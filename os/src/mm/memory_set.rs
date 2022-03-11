/// # 地址空间模块
/// `os/src/mm/memory_set.rs`
/// ## 实现功能
/// ```
/// ```
//

use super::{frame_alloc, FrameTracker};
use super::{PTEFlags, PageTable, PageTableEntry};
use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::{StepByOne, VPNRange};
use crate::config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE};
use crate::sync::UPSafeCell;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::*;
use riscv::register::satp;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

/// ### 地址空间
/// - 符合RAII风格
/// - 一系列有关联的**不一定**连续的逻辑段，这种关联一般是指这些逻辑段组成的虚拟内存空间与一个运行的程序绑定,
/// 即这个运行的程序对代码和数据的直接访问范围限制在它关联的虚拟地址空间之内。
/// 
/// |参数|描述|
/// |--|--|
/// |`page_table`|挂着所有多级页表的节点所在的物理页帧|
/// |`areas`|挂着对应逻辑段中的数据所在的物理页帧|
/// 
/// ```
/// MemorySet::new_bare() -> Self
/// MemorySet::insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission)
/// MemorySet::new_kernel() -> Self
/// ```
pub struct MemorySet {
    /// 挂着所有多级页表的节点所在的物理页帧
    page_table: PageTable,
    /// 挂着对应逻辑段中的数据所在的物理页帧
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// 新建一个空的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// ### 在当前地址空间插入一个 `Framed` 方式映射到物理内存的逻辑段
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    /// ### 在当前地址空间插入一个新的逻辑段
    /// 如果是以 Framed 方式映射到物理内存,
    /// 还可以可选地在那些被映射到的物理页帧上写入一些初始化数据
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            // 写入初始化数据，如果数据存在
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);  // 将生成的数据段压入 areas 使其生命周期由areas控制
    }
    
    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// ### 生成内核的地址空间
    /// - Without kernel stacks.
    /// - 采用恒等映射
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(".bss [{:#x}, {:#x})", sbss_with_stack as usize, ebss as usize);
        println!("mapping .text section");
        // 总体思路：通过Linker.ld中的标签划分内核空间为不同的区块，为每个区块采用恒等映射的方式生成逻辑段，压入地址空间
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        println!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        println!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set
    }

    /// ### 从 ELF 格式可执行文件解析出各数据段并对应生成应用的地址空间
    /// - 返回值
    ///     - Self
    ///     - 用户栈顶地址
    ///     - 程序入口地址
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();
        // 将跳板插入到应用地址空间
        memory_set.map_trampoline();
        // map program headers of elf, with U flag
        // 使用外部 crate xmas_elf 来解析传入的应用 ELF 数据并可以轻松取出各个部分
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        // 取出 ELF 的魔数来判断它是不是一个合法的 ELF
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        // 获取 program header 的数目
        let ph_count = elf_header.pt2.ph_count();
        // 记录目前涉及到的最大的虚拟页号
        let mut max_end_vpn = VirtPageNum(0);
        // 遍历所有的 program header 并对每个 program header 生成一个逻辑段
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(    // 将生成的逻辑段加入到程序地址空间
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        // 分配用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // 在已用最大虚拟页之上放置一个保护页
        user_stack_bottom += PAGE_SIZE; // 栈底
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;   // 栈顶地址
        // 将用户栈加入到程序地址空间
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        // 在应用地址空间中映射次高页面来存放 Trap 上下文
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }
    
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }

    /// ### 根据多级页表和 vpn 查找页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
}

/// ### 逻辑段
/// - 一段虚拟页号连续的区间
/// 
/// |参数|描述|
/// |--|--|
/// |`vpn_range`|描述一段虚拟页号的连续区间，表示该逻辑段在地址区间中的位置和长度
/// |`data_frames`|保存了该逻辑段内的每个虚拟页面和它被映射到的物理页帧的一个键值对容器 BTreeMap 中<br>这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
/// |`map_type`|描述该逻辑段内的所有虚拟页面映射到物理页帧的方式
/// |`map_perm`|控制该逻辑段的访问方式，它是页表项标志位 PTEFlags 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
/// ```
/// MapArea::new(start_va: VirtAddr, end_va: VirtAddr, map_type: MapType, map_perm: MapPermission) -> Self
/// MapArea::map(&mut self, page_table: &mut PageTable)
/// MapArea::unmap(&mut self, page_table: &mut PageTable)
/// MapArea::copy_data(&mut self, page_table: &mut PageTable, data: &[u8])
/// ```
pub struct MapArea {
    /// 描述一段虚拟页号的连续区间，表示该逻辑段在地址区间中的位置和长度
    vpn_range: VPNRange,
    /// 保存了该逻辑段内的每个虚拟页面和它被映射到的物理页帧的一个键值对容器 `BTreeMap` 中<br>
    /// 这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    /// 描述该逻辑段内的所有虚拟页面映射到物理页帧的方式
    map_type: MapType,
    /// 控制该逻辑段的访问方式，它是页表项标志位 PTEFlags 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
    map_perm: MapPermission,
}

impl MapArea {
    /// ### 根据起始 *(会被下取整)* 和终止 *(会被上取整)* 虚拟地址生成一块逻辑段
    /// - 逻辑段大于等于虚拟地址范围
    pub fn new(start_va: VirtAddr, end_va: VirtAddr, map_type: MapType, map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    /// 在多级页表中根据vpn分配空间
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                // 获取一个物理页帧
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                // 将vpn和分配到的物理页帧配对
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        // 在多级页表中建立映射
        page_table.map(vpn, ppn, pte_flags);
    }

    /// 在多级页表中删除指定vpn对应的映射
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    /// ### 在多级页表中为逻辑块分配空间
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    /// 将当前逻辑段到物理内存的映射从传入的该逻辑段所属的地址空间的多级页表中删除
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// ### 将切片 `data` 中的数据拷贝到当前逻辑段实际被内核放置在的各物理页帧上
    /// - 切片 `data` 中的数据大小不超过当前逻辑段的总大小，且切片中的数据会被对齐到逻辑段的开头，然后逐页拷贝到实际的物理页帧
    /// - 只有 `Framed` 可以被拷贝
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            // 每次取出 4KiB 大小的数据
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

/// ### 虚拟页面映射到物理页帧的方式
/// - `Identical`: 恒等映射，一般用在内核空间（空间已分配）
/// - `Framed`: 新分配一个物理页帧
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    /// 恒等映射，一般用在内核空间（空间已分配）
    Identical,
    ///对于每个虚拟页面都有一个新分配的物理页帧与之对应，虚地址与物理地址的映射关系是相对随机的
    Framed,
}

bitflags! {
    /// 页表项标志位 `PTE Flags` 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(
        !kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .writable(),
    );
    assert!(
        !kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .writable(),
    );
    assert!(
        !kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .executable(),
    );
    println!("remap_test passed!");
}
