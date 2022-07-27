/// # 地址空间模块
/// `os/src/mm/memory_set.rs`
/// ## 实现功能
/// ```
/// pub static ref KERNEL_SPACE: Arc<UP&SafeCell<MemorySet>>
/// pub struct MemorySet
/// pub struct MapArea
/// ```
//
use super::{frame_alloc, FrameTracker};
use super::{PTEFlags, PageTable, PageTableEntry};
use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::{StepByOne};
use crate::config::*;
use crate::mm::frame_usage;
use crate::sync::UPSafeCell;
use crate::task::{AuxEntry, AT_ENTRY, AT_PHDR, AT_PHENT, AT_PHNUM, AT_BASE};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::*;
use riscv::register::satp;

// 动态链接部分
use crate::fs::{OpenFlags,open};

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
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

pub fn kernel_token() -> usize {
    KERNEL_SPACE.exclusive_access().token()
}

/// ### 地址空间
///
/// |参数|描述|
/// |--|--|
/// |`page_table`|页表|
/// |`chunks`|逻辑段|
/// |`stack_chunks`|内核栈|
///
/// ```
/// MemorySet::new_bare() -> Self
/// MemorySet::insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission)
/// MemorySet::new_kernel() -> Self
/// ```
pub struct MemorySet {
    /// 页表
    page_table: PageTable,
    /// 逻辑段
    chunks: Vec<MapArea>,
    // /// 内核栈
    // stack_chunks: MapArea,
}

impl MemorySet {
    /// 新建一个空的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            chunks: Vec::new(),
        }
    }

    /// 获取当前页表的 token (符合 satp CSR 格式要求的多级页表的根节点所在的物理页号)
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// 在当前地址空间插入一个 `Framed` 方式映射到物理内存的逻辑段
    pub fn insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
        self.push(MapArea::new(start_va, end_va, MapType::Framed, permission), None);
    }

    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, chunk)) = self
            .chunks
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.start_va.floor() == start_vpn)
        {
            chunk.unmap(&mut self.page_table);
            self.chunks.remove(idx);
        }
    }

    /// ### 在当前地址空间插入一个新的逻辑段
    /// - 如果是以 Framed 方式映射到物理内存,
    /// 还可以可选地在那些被映射到的物理页帧上写入一些初始化数据
    /// - 对于 `Identical`，在 `push` 的时候完成多级页表的建立
    /// - 对于`Framed`， 在 `copy_data` 时按需分配空间
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        if map_area.map_type == MapType::Identical {
            let mut vpn = map_area.start_va.floor();
            let end_vpn = map_area.end_va.floor();
            while vpn != end_vpn {                 
                map_area.push_vpn(vpn, &mut self.page_table);
                vpn.step();
            }
        }
        if let Some(data) = data {
            // 写入初始化数据，如果数据存在
            map_area.copy_data(&mut self.page_table, data);
        }
        self.chunks.push(map_area); // 将生成的数据段压入 chunks 使其生命周期由 chunks 控制
    }

    /// 映射跳板的虚拟页号和物理物理页号
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
        println!("mapping memory-mapped registers");
        for pair in MMIO {
            // 恒等映射 内存映射 I/O (MMIO, Memory-Mapped I/O) 地址到内核地址空间
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }
        memory_set
    }

    /// ### 从 ELF 格式可执行文件解析出各数据段并对应生成应用的地址空间
    /// - 返回值
    ///     - Self(MemorySet)
    ///     - 用户栈顶地址
    ///     - 用户堆底地址
    ///     - 程序入口地址
    pub fn from_elf(elf_data: &[u8], auxs: &mut Vec<AuxEntry>) -> (Self, usize, usize, usize) {
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
        // 是否为动态加载
        let mut elf_interpreter = false;
        // 动态加载器入口地址
        let mut interpreter_entry = 0;

        // 遍历程序段进行加载
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            match ph.get_type().unwrap() {
                xmas_elf::program::Type::Phdr => auxs.push(AuxEntry(AT_PHDR, ph.virtual_addr() as usize)),
                xmas_elf::program::Type::Interp => {
                    // 加入解释器需要的 aux 字段
                    auxs.push(AuxEntry(AT_PHENT, elf_header.pt2.ph_entry_size().into()));
                    auxs.push(AuxEntry(AT_PHNUM, ph_count.into()));
                    auxs.push(AuxEntry(AT_ENTRY, elf_header.pt2.entry_point() as usize));
                    elf_interpreter = true;
                }
                xmas_elf::program::Type::Load => {
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
                    max_end_vpn = map_area.end_va.floor();
                    // println!("start_va:0x{:x},end_va:0x{:x}",start_va.0,end_va.0);
                    // println!("offset:0x{:x},file_size:0x{:x}",ph.offset(),ph.file_size());
                    memory_set.push(
                        // 将生成的逻辑段加入到程序地址空间
                        map_area,
                        Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                    );
                    // let t_current = FRAME_ALLOCATOR.exclusive_access().current;
                    // println!("current:0x{:x}",t_current);
                }
                _ => continue,
            }
        }
        if elf_interpreter {
            // 动态链接
            let interp = open("/", "libc.so", OpenFlags::O_RDONLY).unwrap();
            let interp_data = interp.read_all();
            let interp_elf = xmas_elf::ElfFile::new(interp_data.as_slice()).unwrap();
            let interp_elf_header = interp_elf.header;
            let base_address = 0x200000;
            interpreter_entry = interp_elf_header.pt2.entry_point() as usize + base_address;
            auxs.push(AuxEntry(AT_BASE, interpreter_entry));
            // 获取 program header 的数目
            println!("[info] begin map interp");
            let ph_count = interp_elf_header.pt2.ph_count();
            for i in 0..ph_count{
                let ph = interp_elf.program_header(i).unwrap();
                match ph.get_type().unwrap() {
                    xmas_elf::program::Type::Load => {
                        let start_va: VirtAddr = (ph.virtual_addr() as usize+ base_address).into() ;
                        let end_va: VirtAddr = (ph.virtual_addr() as usize+ ph.mem_size() as usize+ base_address).into();
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
                        // println!("start_va:0x{:x},end_va:0x{:x}",start_va.0,end_va.0);
                        // println!("vpnrange:{:?}",map_area.vpn_range);
                        // println!("offset:0x{:x},file_size:0x{:x}",ph.offset(),ph.file_size());
                        memory_set.push(
                            // 将生成的逻辑段加入到程序地址空间
                            map_area,
                            Some(&interp_elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                        );
                    }
                    _ => continue,
                }

            }
        }
        // 分配用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // 在已用最大虚拟页之上放置一个保护页
        user_stack_bottom += PAGE_SIZE; // 栈底
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE; // 栈顶地址
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
        // 分配用户堆
        let mut user_heap_bottom: usize = user_stack_top;
        //放置一个保护页
        user_heap_bottom += PAGE_SIZE;
        let user_heap_top: usize = user_heap_bottom + USER_HEAP_SIZE;

        memory_set.push(
            MapArea::new(
                user_heap_bottom.into(),
                user_heap_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        if elf_interpreter {
            (memory_set, user_stack_top, user_heap_bottom, interpreter_entry)
        } else {
            (memory_set, user_stack_top, user_heap_bottom, elf_header.pt2.entry_point() as usize)
        }
    }

    /// 复制一个完全相同的地址空间
    pub fn from_existed_user(user_space: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();
        // 映射跳板
        memory_set.map_trampoline();
        // 循环拷贝每一个逻辑段到新的地址空间
        for area in user_space.chunks.iter() {
            let mut new_area = MapArea::from_another(area);
            if area.map_type == MapType::Framed {
                // 注意：对于 Framed 按物理页帧拷贝数据，需要在获取 dst_ppn 前进行内存分配
                for vpn in &area.vpn_table {
                    new_area.push_vpn(*vpn, &mut memory_set.page_table);
                    let src_ppn = user_space.translate(*vpn).unwrap().ppn();
                    let dst_ppn = memory_set.translate(*vpn).unwrap().ppn();
                    dst_ppn.get_bytes_array().copy_from_slice(src_ppn.get_bytes_array());
                }
                memory_set.push(new_area, None);
            }else{
                memory_set.push(new_area, None);
                // 按物理页帧拷贝数据
                for vpn in &area.vpn_table {
                    let src_ppn = user_space.translate(*vpn).unwrap().ppn();
                    let dst_ppn = memory_set.translate(*vpn).unwrap().ppn();
                    dst_ppn.get_bytes_array().copy_from_slice(src_ppn.get_bytes_array());
                }
            }
        }
        memory_set
    }

    /// 为mmap缺页分配空页表
    pub fn lazy_mmap(&mut self, stval: VirtAddr) -> isize {
        for chunk in self.chunks.iter_mut() {
            if stval >= chunk.start_va && stval < chunk.end_va {
                chunk.push_vpn(stval.floor(), &mut self.page_table);
                return 0;
            }
        }
        -1
    }

    /// ### 激活当前虚拟地址空间
    /// 将多级页表的token（格式化后的root_ppn）写入satp
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma"); // 将快表清空
        }
    }

    /// 根据多级页表和 vpn 查找页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    // WARNING: This function causes inconsistency between pte flags and
    //          map_area flags.
    // return -1 if not found, 0 if found
    pub fn set_pte_flags(&mut self, vpn: VirtPageNum, flags: usize) -> isize {
        self.page_table.set_pte_flags(vpn, flags)
    }

    /// ### 回收应用地址空间
    /// 将地址空间中的逻辑段列表 chunks 清空（即执行 Vec 向量清空），<br>
    /// 这将导致应用地址空间被回收（即进程的数据和代码对应的物理页帧都被回收），<br>
    /// 但用来存放页表的那些物理页帧此时还不会被回收（会由父进程最后回收子进程剩余的占用资源）
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.chunks.clear();
    }

    /// ### 在地址空间中插入一个 mmap 逻辑段
    /// - 已确定：
    ///     - 起止虚拟地址
    ///     - 映射方式：Framed
    ///     - map_perm
    /// - 留空：
    ///     - vpn_table
    ///     - data_frames
    pub fn insert_mmap_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
        let new_chunk_area = MapArea::new(start_va, end_va, MapType::Framed, permission);
        self.chunks.push(new_chunk_area);
    }

    #[allow(unused)]
    pub fn debug_show_data(&self, va: VirtAddr) {
        println!("-----------------------PTE Data-----------------------");
        println!("MemorySet token: 0x{:x}", self.token());
        let findpte = self.translate(va.floor());
        if let Some(pte) = findpte {
            println!("VirtAddr 0x{:x} ", va.0);
            println!("ppn:     0x{:x}XXX", pte.ppn().0);
            println!("pte_raw: 0b{:b}", pte.bits);
            println!("executable: {}", pte.executable());
            println!("readable:   {}", pte.readable());
            println!("writable:   {}", pte.writable());
        } else {
            println!("VirtAddr 0x{:x} is not valied", va.0);
            println!("------------------------------------------------------");
            return;
        }
        println!("------------------------------------------------------");

        unsafe {
            let pa = findpte.unwrap().ppn().0 << 12;
            let raw_data = core::slice::from_raw_parts(pa as *const usize, 512);
            let mut i = 0;
            while i < 512 {
                print!("offset:{:03x}\t0x{:016x}", (i) * 8, raw_data[i]);
                print!("\t");
                print!("offset:{:03x}\t0x{:016x}", (i + 1) * 8, raw_data[i + 1]);
                print!("\t");
                print!("offset:{:03x}\t0x{:016x}", (i + 2) * 8, raw_data[i + 2]);
                print!("\t");
                println!("offset:{:03x}\t0x{:016x}", (i + 3) * 8, raw_data[i + 3]);
                i += 4;
            }
        }
    }
}

/// ### 虚拟页面映射到物理页帧的方式
/// |内容|描述|
/// |--|--|
/// |`Identical`|恒等映射，一般用在内核空间（空间已分配）|
/// |`Framed`|新分配一个物理页帧|
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

/// ### 逻辑段
///
/// |参数|描述|
/// |--|--|
/// |`vpn_table`|逻辑段中已分配物理内存的页号
/// |`data_frames`|键值对容器 BTreeMap ,保存了该逻辑段内的每个虚拟页面的 VPN 和被映射到的物理页帧<br>这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
/// |`map_type`|描述该逻辑段内的所有虚拟页面映射到物理页帧的方式
/// |`map_perm`|控制该逻辑段的访问方式，它是页表项标志位 PTEFlags 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
/// |`start_va`|逻辑段起始虚拟地址
/// |`end_va`|逻辑段结束虚拟地址
/// |`fd`|存放数据的文件描述符
/// |`offset`|文件偏移量
/// |`length`|文件中数据长度
/// ```
/// MapArea::new() -> Self
/// MapArea::from_another() -> Self
/// MapArea::map()
/// MapArea::unmap()
/// MapArea::copy_data()
/// MapArea::push_vpn()
/// ```
pub struct MapArea {
    /// 逻辑段中已分配物理内存的页号
    vpn_table: Vec<VirtPageNum>,
    /// 键值对容器 BTreeMap ,保存了该逻辑段内的每个虚拟页面的 VPN 和被映射到的物理页帧<br>
    /// 这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    /// 描述该逻辑段内的所有虚拟页面映射到物理页帧的方式
    map_type: MapType,
    /// 控制该逻辑段的访问方式，它是页表项标志位 PTEFlags 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
    map_perm: MapPermission,
    /// 逻辑段起始虚拟地址
    start_va: VirtAddr,
    /// 逻辑段结束虚拟地址
    end_va: VirtAddr,
}

impl MapArea {
    /// ### 根据起始 *(会被下取整)* 和终止 *(会被上取整)* 虚拟地址生成一块逻辑段
    /// - 逻辑段大于等于虚拟地址范围
    pub fn new(start_va: VirtAddr, end_va: VirtAddr, map_type: MapType, map_perm: MapPermission) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            start_va,
            end_va,
        }
    }

    /// ### 从一个逻辑段复制得到一个虚拟地址区间、映射方式和权限控制均相同的逻辑段
    /// 不同的是由于它还没有真正被映射到物理页帧上，所以 `vpn_table` 和 `data_frames` 字段为空
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_table: another.vpn_table.clone(),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            start_va: another.start_va,
            end_va: another.end_va,
        }
    }

    /// 在多级页表中根据vpn分配空间
    fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                // 获取一个物理页帧
                let frame = frame_alloc().unwrap();      
                ppn = frame.ppn;
                // println!("current vpn:0x{:x},get ppn:0x{:x}",vpn.0,ppn.0);
                // 将vpn和分配到的物理页帧配对
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        // 在多级页表中建立映射
        page_table.map(vpn, ppn, pte_flags);
    }

    /// 将当前逻辑段到物理内存的映射从传入的该逻辑段所属的地址空间的多级页表中删除
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        while !self.vpn_table.is_empty() {
            let vpn = self.vpn_table[0];
            if self.map_type == MapType::Framed {
                self.data_frames.remove(&vpn);
            }
            page_table.unmap(vpn);
            assert_eq!(self.vpn_table.remove(0),vpn);
        }
    }

    /// ### 将切片 `data` 中的数据拷贝到当前逻辑段实际被内核放置在的各物理页帧上
    /// - 切片 `data` 中的数据大小不超过当前逻辑段的总大小，且切片中的数据会被对齐到逻辑段的开头，然后逐页拷贝到实际的物理页帧
    /// - 只有 `Framed` 可以被拷贝
    /// - 真实的内存空间只有在需要填入数据时才会分配给进程
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.start_va.floor();
        let mut len = data.len();

        // 使对齐
        let first_page_offset = self.start_va.0 % PAGE_SIZE; // 起始页中偏移
        if first_page_offset != 0 {
            let first_page_write_len = (PAGE_SIZE - first_page_offset).min(data.len()); // 起始页中需要写入的长度
            let src = &data[start..first_page_write_len];
            let dst = &mut page_table.translate(current_vpn)
                .unwrap_or_else(
                    // 对于全局lazy的情况，要考虑物理内存还没被分配便需要拷贝数据的情况，即查找失败
                    || {
                        self.push_vpn(current_vpn, page_table);
                        page_table.translate(current_vpn).unwrap()  
                    }
                )
                .ppn()
                .get_bytes_array()
                [first_page_offset..first_page_offset + first_page_write_len];
            dst.copy_from_slice(src);
            start += first_page_write_len;
            current_vpn.step();
            len -= first_page_write_len;
        }
        // 后续拷贝
        loop {
            // 每次取出 4KiB 大小的数据
            println!("1");
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table.translate(current_vpn)
                .unwrap_or_else(    // 查找失败，分配一页后再写入数据
                    || {
                        self.push_vpn(current_vpn, page_table);
                        page_table.translate(current_vpn).unwrap()  
                    }
                )
                .ppn()
                .get_bytes_array()[..src.len()];
            println!("2");
            dst.copy_from_slice(src);
            println!("3");
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }

    /// 为指定虚拟页分配物理空间，并加入逻辑段
    pub fn push_vpn(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
        self.vpn_table.push(vpn);
        self.map_one(page_table, vpn);
    }
}
