/// # 地址空间模块
/// `os/src/mm/memory_set.rs`
/// ## 实现功能
/// ```
/// pub static ref KERNEL_SPACE: Arc<UP&SafeCell<MemorySet>>
/// pub struct MemorySet
/// pub struct MapArea
/// ```
//

use super::frame_allocator::{frame_alloc, FrameTracker, frame_add_ref, enquire_refcount};
use super::page_table::{PTEFlags, PageTable, PageTableEntry};
use super::address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum, VPNRange};
use crate::config::*;
use crate::fs::OSInode;
use crate::task::{AuxEntry, AT_ENTRY, AT_PHDR, AT_PHENT, AT_PHNUM};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::*;
use riscv::register::satp;
use spin::Mutex;

// 动态链接部分

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
    pub static ref KERNEL_SPACE: Arc<Mutex<MemorySet>> = Arc::new(Mutex::new(MemorySet::new_kernel()));
}

pub fn kernel_token() -> usize {
    KERNEL_SPACE.lock().token()
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
    heap_chunk: ChunkArea,
    mmap_chunks: Vec<ChunkArea>,
}

impl MemorySet {
    /// 新建一个空的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
            heap_chunk: ChunkArea::new(
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
                0.into(),
                0.into()),
            mmap_chunks: Vec::new(),
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

    /// 通过起始虚拟页号删除对应的逻辑段（包括连续逻辑段和离散逻辑段）
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }

        if let Some((idx, chunk)) = self
            .mmap_chunks
            .iter_mut()
            .enumerate()
            .find(|(_, chunk)| chunk.start_va.floor() == start_vpn)
        {
            chunk.unmap(&mut self.page_table);
            self.mmap_chunks.remove(idx);
        }
    }

    /// ### 在当前地址空间插入一个新的连续逻辑段
    /// - 物理页号是随机分配的
    /// - 如果是以 Framed 方式映射到物理内存,
    /// 还可以可选性地在那些被映射到的物理页帧上写入一些初始化数据
    /// - data:(osinode,offset,len,page_offset)
    fn push(&mut self, mut map_area: MapArea, data: Option<(Arc<OSInode>, usize,usize, usize)>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            // 写入初始化数据，如果数据存在
            map_area.copy_data(&mut self.page_table, data.0, data.1, data.2, data.3);
        }
        self.areas.push(map_area); // 将生成的数据段压入 areas 使其生命周期由areas控制
    }

    /// ### 在当前地址空间插入一段已被分配空间的连续逻辑段
    /// 主要用于 COW 创建时子进程空间连续逻辑段的插入，其要求指定物理页号
    fn push_mapped_area(&mut self, map_area: MapArea) {
        self.areas.push(map_area);
    }

    /// 映射跳板的虚拟页号和物理物理页号
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    fn map_trap_context(&mut self) {
        self.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
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
        // 总体思路：通过Linker.ld中的标签划分内核空间为不同的区块，为每个区块采用恒等映射的方式生成逻辑段，压入地址空间
        println!("mapping .text section");
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
    ///     - Self
    ///     - 用户栈顶地址
    ///     - 程序入口地址
    pub fn load_elf(elf_file: Arc<OSInode>, auxs: &mut Vec<AuxEntry>) -> (Self, usize, usize, usize) {
        let mut memory_set = Self::new_bare();
        // 将跳板插入到应用地址空间
        memory_set.map_trampoline();
        // 在应用地址空间中映射次高页面来存放 Trap 上下文
        // 将 TRAP_CONTEXT 段尽量放前，以节省 cow 时寻找时间
        memory_set.map_trap_context();

        // 第一次读取前64字节确定程序表的位置与大小
        let elf_head_data = elf_file.read_vec(0, 64);
        let elf = xmas_elf::ElfFile::new(elf_head_data.as_slice()).unwrap();

        let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
        let ph_offset = elf.header.pt2.ph_offset() as usize;
        let ph_count = elf.header.pt2.ph_count() as usize;

        // 进行第二次读取，这样的elf对象才能正确解析程序段头的信息
        let elf_head_data = elf_file.read_vec(0, ph_offset + ph_count * ph_entry_size);
        let elf = xmas_elf::ElfFile::new(elf_head_data.as_slice()).unwrap();

        // 记录目前涉及到的最大的虚拟页号
        let mut max_end_vpn = VirtPageNum(0);
        // 是否为动态加载
        let mut elf_interpreter = false;
        // 动态链接器加载地址
        let mut interp_entry_point = 0;
        // 遍历程序段进行加载
        for i in 0..ph_count as u16 {
            let ph = elf.program_header(i).unwrap();
            match ph.get_type().unwrap() {
                xmas_elf::program::Type::Phdr => auxs.push(AuxEntry(AT_PHDR, ph.virtual_addr() as usize)),
                xmas_elf::program::Type::Interp => {
                    // 加入解释器需要的 aux 字段
                    auxs.push(AuxEntry(AT_PHENT, elf.header.pt2.ph_entry_size().into()));
                    auxs.push(AuxEntry(AT_PHNUM, ph_count.into()));
                    auxs.push(AuxEntry(AT_ENTRY, elf.header.pt2.entry_point() as usize));
                    elf_interpreter = true;
                }
                xmas_elf::program::Type::Load => {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                    let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                    let map_perm = MapPermission::U | MapPermission::R | MapPermission::W | MapPermission::X;
                    // let mut map_perm = MapPermission::U;
                    // let ph_flags = ph.flags();
                    // if ph_flags.is_read() {
                    //     map_perm |= MapPermission::R;
                    // }
                    // if ph_flags.is_write() {
                    //     map_perm |= MapPermission::W;
                    // }
                    // if ph_flags.is_execute() {
                    //     map_perm |= MapPermission::X;
                    // }
                    let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                    // println!("start va:0x{:x}, end va:0x{:x}",start_va.0,end_va.0);
                    // println!("{:?}",map_area.vpn_range);

                    max_end_vpn = map_area.vpn_range.get_end();
                    memory_set.push(map_area, Some((elf_file.clone(), ph.offset() as usize, ph.file_size() as usize, start_va.page_offset())));
                }
                _ => continue,
            }
        }
        // if elf_interpreter {
        //     // 动态链接
        //     let interp_elf = xmas_elf::ElfFile::new(LIBC_SO.as_slice()).unwrap();
        //     let interp_elf_header = interp_elf.header;
        //     let base_address = 0x2000000000;
        //     auxs.push(AuxEntry(AT_BASE, base_address));
        //     interp_entry_point = base_address + interp_elf_header.pt2.entry_point() as usize;
        //     // 获取 program header 的数目
        //     let ph_count = interp_elf_header.pt2.ph_count();
        //     for i in 0..ph_count {
        //         let ph = interp_elf.program_header(i).unwrap();
        //         if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
        //             let start_va: VirtAddr = (ph.virtual_addr() as usize + base_address).into();
        //             let end_va: VirtAddr = (ph.virtual_addr() as usize + ph.mem_size() as usize + base_address).into();
        //             let map_perm = MapPermission::U | MapPermission::R | MapPermission::W | MapPermission::X;
        //             unimplemented!("[Kernel] elf_interpreter data loading needs rewrite");
        //             // memory_set.push(
        //             //     MapArea::new(start_va, end_va, MapType::Framed, map_perm),
        //             //     Some(&interp_elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
        //             //     start_va.page_offset(),
        //             // );
        //         }
        //     }
        // } else {
        //     auxs.push(AuxEntry(AT_BASE, 0));
        // }

        // 分配用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into(); // 栈底
        user_stack_bottom += PAGE_SIZE; // 在已用最大虚拟页之上放置一个保护页
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE; // 栈顶地址
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // 分配用户堆
        let mut user_heap_bottom: usize = user_stack_top;
        user_heap_bottom += PAGE_SIZE; //放置一个保护页
        let user_heap_top: usize = user_heap_bottom + USER_HEAP_SIZE;
        memory_set.heap_chunk = ChunkArea::new(
            MapType::Framed,
            MapPermission::R | MapPermission::W | MapPermission::U,
            user_heap_bottom.into(),
            user_heap_top.into(),
        );
        
        if elf_interpreter {
            (memory_set, user_stack_top, user_heap_bottom, interp_entry_point)
        } else {
            (memory_set, user_stack_top, user_heap_bottom, elf.header.pt2.entry_point() as usize)
        }
    }

    /// 以COW的方式复制一个地址空间
    pub fn from_copy_on_write(user_space: &mut MemorySet) -> MemorySet {
        let mut new_memory_set = Self::new_bare();

        // This part is not for Copy on Write.
        // Including:   Trampoline
        //              Trap_Context
        new_memory_set.map_trampoline();
        for area in user_space.areas.iter() {
            let start_vpn = area.vpn_range.get_start();
            if start_vpn == VirtAddr::from(TRAP_CONTEXT).floor()  {
                let new_area = MapArea::from_another(area);
                new_memory_set.push(new_area, None);
                for vpn in area.vpn_range {
                    let src_ppn = user_space.translate(vpn).unwrap().ppn();
                    let dst_ppn = new_memory_set.translate(vpn).unwrap().ppn();
                    // println!{"[COW TRAP_CONTEXT] mapping {:?} --- {:?}, src: {:?}", vpn, dst_ppn, src_ppn};
                    dst_ppn.get_bytes_array().copy_from_slice(src_ppn.get_bytes_array());
                }
            }
            break;
        }
        //This part is for copy on write
        let parent_areas = & user_space.areas;
        let parent_page_table = &mut user_space.page_table;
        for area in parent_areas.iter() {
            let start_vpn = area.vpn_range.get_start();
            if start_vpn != VirtAddr::from(TRAP_CONTEXT).floor() {
                let mut new_area = MapArea::from_another(area);
                // map the former physical address
                for vpn in area.vpn_range {
                    // change the map permission of both pagetable
                    // get the former flags and ppn
                    let pte = parent_page_table.translate(vpn).unwrap();
                    let pte_flags = pte.flags() & !PTEFlags::W;
                    let src_ppn = pte.ppn();
                    frame_add_ref(src_ppn);
                    // change the flags of the src_pte
                    parent_page_table.set_flags(vpn, pte_flags);
                    parent_page_table.set_cow(vpn);
                    // map the cow page table to src_ppn
                    new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                    new_memory_set.page_table.set_cow(vpn);
                    new_area.insert_tracker(vpn, src_ppn);
                }
                new_memory_set.push_mapped_area(new_area);
            }
        }
        for chunk in user_space.mmap_chunks.iter() {
            let mut new_chunk = ChunkArea::from_another(chunk);
            for _vpn in chunk.vpn_table.iter() {
                let vpn = (*_vpn).clone();
                // change the map permission of both pagetable
                // get the former flags and ppn
                let pte = parent_page_table.translate(vpn).unwrap();
                let pte_flags = pte.flags() & !PTEFlags::W;
                let src_ppn = pte.ppn();
                frame_add_ref(src_ppn);
                // change the flags of the src_pte
                parent_page_table.set_flags(vpn, pte_flags);
                parent_page_table.set_cow(vpn);
                // map the cow page table to src_ppn
                new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                new_memory_set.page_table.set_cow(vpn);
                new_chunk.vpn_table.push(vpn);
                new_chunk.insert_tracker(vpn, src_ppn);
            }
            new_memory_set.mmap_chunks.push(new_chunk);
        }
        new_memory_set.heap_chunk = ChunkArea::from_another(&user_space.heap_chunk);
        for _vpn in user_space.heap_chunk.vpn_table.iter() {
            let vpn = (*_vpn).clone();
            // change the map permission of both pagetable
            // get the former flags and ppn
            let pte = parent_page_table.translate(vpn).unwrap();
            let pte_flags = pte.flags() & !PTEFlags::W;
            let src_ppn = pte.ppn();
            frame_add_ref(src_ppn);
            // change the flags of the src_pte
            parent_page_table.set_flags(vpn, pte_flags);
            parent_page_table.set_cow(vpn);
            // map the cow page table to src_ppn
            new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
            new_memory_set.page_table.set_cow(vpn);
            new_memory_set.heap_chunk.vpn_table.push(vpn);
            new_memory_set.heap_chunk.insert_tracker(vpn, src_ppn);
        }
        // new_memory_set.debug_show_layout();
        new_memory_set
    }

    #[no_mangle]
    pub fn cow_alloc(&mut self, vpn: VirtPageNum, former_ppn: PhysPageNum) -> isize {
        if enquire_refcount(former_ppn) == 1 {
            self.page_table.reset_cow(vpn);
            // change the flags of the src_pte
            self.page_table.set_flags(
                vpn, 
                self.page_table.translate(vpn).unwrap().flags() | PTEFlags::W
            );
            return 0
        }
        let frame = frame_alloc().unwrap();
        let ppn = frame.ppn;
        self.remap_cow(vpn, ppn, former_ppn);
        for area in self.areas.iter_mut() {
            let head_vpn = area.vpn_range.get_start();
            let tail_vpn = area.vpn_range.get_end();
            if vpn <= tail_vpn && vpn >= head_vpn {
                area.data_frames.insert(vpn, frame);
                return 0;
            }
        }
        for chunk in self.mmap_chunks.iter_mut() {
            let head_vpn = VirtPageNum::from(chunk.start_va);
            let tail_vpn = VirtPageNum::from(chunk.end_va);
            if vpn <= tail_vpn && vpn >= head_vpn {
                chunk.data_frames.insert(vpn, frame);
                return 0;
            }
        }
        let head_vpn = VirtPageNum::from(self.heap_chunk.start_va);
        let tail_vpn = VirtPageNum::from(self.heap_chunk.end_va);
        if vpn <= tail_vpn && vpn >= head_vpn {
            self.heap_chunk.data_frames.insert(vpn, frame);
            return 0;
        }
        0
    }

    fn remap_cow(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, former_ppn: PhysPageNum) {
        self.page_table.remap_cow(vpn, ppn, former_ppn);
    }

    /// 为mmap缺页分配空页表
    pub fn lazy_mmap(&mut self, stval: VirtAddr) -> isize {
        for mmap_chunk in self.mmap_chunks.iter_mut() {
            if stval >= mmap_chunk.start_va && stval < mmap_chunk.end_va {
                mmap_chunk.push_vpn(stval.floor(), &mut self.page_table);
                return 0;
            }
        }
        -1
    }

    pub fn lazy_alloc_heap (&mut self, vpn: VirtPageNum) -> isize {
        self.heap_chunk.push_vpn(vpn, &mut self.page_table);
        0
    }

    // pub fn lazy_alloc_stack (&mut self, vpn: VirtPageNum) -> isize {
    //     self.stack_chunk.push_vpn(vpn, &mut self.page_table);
    //     0
    // }

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
    /// 将地址空间中的逻辑段列表 areas 清空（即执行 Vec 向量清空），
    /// 这将导致应用地址空间被回收（即进程的数据和代码对应的物理页帧都被回收），
    /// 但用来存放页表的那些物理页帧此时还不会被回收（会由父进程最后回收子进程剩余的占用资源）
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.areas.clear();
    }

    /// ### 在地址空间中插入一个空的离散逻辑段
    /// - 已确定：
    ///     - 起止虚拟地址
    ///     - 映射方式：Framed
    ///     - map_perm
    /// - 留空：
    ///     - vpn_table
    ///     - data_frames
    pub fn insert_mmap_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
        let new_chunk_area = ChunkArea::new(MapType::Framed, permission, start_va, end_va);
        self.mmap_chunks.push(new_chunk_area);
    }

    #[allow(unused)]
    pub fn debug_show_data(&self, va: VirtAddr) {
        println!("-----------------------PTE Data-----------------------");
        println!("MemorySet token: 0x{:x}", self.token());
        let findpte = self.translate(va.floor());
        if let Some(pte) = findpte {
            println!("VirtAddr 0x{:x} ", va.0);
            println!("ppn:     0x{:x}---", pte.ppn().0);
            println!("pte_raw: 0b{:b}", pte.bits);
            println!("valid   :   {}", pte.is_valid());
            println!("executable: {}", pte.executable());
            println!("readable:   {}", pte.readable());
            println!("writable:   {}", pte.writable());
            println!("COW     :   {}", pte.is_cow());
        } else {
            println!("VirtAddr 0x{:x} is not valied", va.0);
            println!("------------------------------------------------------");
            return;
        }
        println!("------------------------------------------------------");

        if let Some(pte) = findpte{
            if pte.is_valid() {
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
            } else {
                println!("VirtAddr 0x{:x} is not valied", va.0);
                println!("------------------------------------------------------");
                return;
            }
        }
        
    }

    #[allow(unused)]
    pub fn debug_show_layout(&self) {
        println!("-----------------------MM Layout-----------------------");
        for area in &self.areas {
            print!(
                "MapArea  : 0x{:010x}--0x{:010x} len:0x{:08x} ",
                area.start_va.0,
                area.end_va.0,
                area.end_va.0 - area.start_va.0
            );
            if area.map_perm.is_user() {
                print!("U");
            } else {
                print!("-");
            };
            if area.map_perm.is_read() {
                print!("R");
            } else {
                print!("-");
            };
            if area.map_perm.is_write() {
                print!("W");
            } else {
                print!("-");
            };
            if area.map_perm.is_execute() {
                println!("X");
            } else {
                println!("-");
            };
        }
        for mmap_chunk in &self.mmap_chunks {
            print!(
                "ChunkArea: 0x{:010x}--0x{:010x} len:0x{:08x} ",
                mmap_chunk.start_va.0,
                mmap_chunk.end_va.0,
                mmap_chunk.end_va.0 - mmap_chunk.start_va.0
            );
            if mmap_chunk.map_perm.is_user() {
                print!("U");
            } else {
                print!("-");
            };
            if mmap_chunk.map_perm.is_read() {
                print!("R");
            } else {
                print!("-");
            };
            if mmap_chunk.map_perm.is_write() {
                print!("W");
            } else {
                print!("-");
            };
            if mmap_chunk.map_perm.is_execute() {
                println!("X");
            } else {
                println!("-");
            };
        }
        print!(
            "HeapArea : 0x{:010x}--0x{:010x} len:0x{:08x} ",
            self.heap_chunk.start_va.0,
            self.heap_chunk.end_va.0,
            self.heap_chunk.end_va.0 - self.heap_chunk.start_va.0
        );
        if self.heap_chunk.map_perm.is_user() {
            print!("U");
        } else {
            print!("-");
        };
        if self.heap_chunk.map_perm.is_read() {
            print!("R");
        } else {
            print!("-");
        };
        if self.heap_chunk.map_perm.is_write() {
            print!("W");
        } else {
            print!("-");
        };
        if self.heap_chunk.map_perm.is_execute() {
            println!("X");
        } else {
            println!("-");
        };
        println!("-------------------------------------------------------");
    }
}

/// ### 离散逻辑段
pub struct ChunkArea {
    vpn_table: Vec<VirtPageNum>,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
    start_va: VirtAddr,
    end_va: VirtAddr,
}

impl ChunkArea {
    pub fn new(map_type: MapType, map_perm: MapPermission, start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            start_va: start,
            end_va: end,
        }
    }
    pub fn push_vpn(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
        self.vpn_table.push(vpn);
        self.map_one(page_table, vpn);
    }
    pub fn from_another(another: &ChunkArea) -> Self {
        Self {
            vpn_table: Vec::new(),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            start_va: another.start_va,
            end_va: another.end_va,
        }
    }
    // Alloc and map one page
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                if let Some(frame) = frame_alloc() {
                    ppn = frame.ppn;
                    self.data_frames.insert(vpn, frame);
                } else {
                    panic!("No more memory!");
                }
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }

    // Alloc and map all pages
    // pub fn map(&mut self, page_table: &mut PageTable) {
    //     for vpn in self.vpn_table {
    //         self.map_one(page_table, vpn);
    //     }
    // }
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_table.clone() {
            self.unmap_one(page_table, vpn);
        }
    }

    pub fn insert_tracker(&mut self, vpn: VirtPageNum, ppn: PhysPageNum) {
        self.data_frames.insert(vpn, FrameTracker::from_ppn(ppn));
    }
}

// 虚拟页面映射到物理页帧的方式
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical, // 恒等映射，一般用在内核空间（空间已分配）
    Framed, // 对于每个虚拟页面都有一个新分配的物理页帧与之对应，虚地址与物理地址的映射关系是相对随机的
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

impl MapPermission {
    pub fn is_read(self) -> bool {
        self.bits & 1 << 1 == 1 << 1
    }
    pub fn is_write(self) -> bool {
        self.bits & 1 << 2 == 1 << 2
    }
    pub fn is_execute(self) -> bool {
        self.bits & 1 << 3 == 1 << 3
    }
    pub fn is_user(self) -> bool {
        self.bits & 1 << 4 == 1 << 4
    }
}

/// ### 连续逻辑段
/// - 一段虚拟页号连续的区间
///
/// |参数|描述|
/// |--|--|
/// |`vpn_range`|描述一段虚拟页号的连续区间，表示该逻辑段在地址区间中的位置和长度
/// |`data_frames`|键值对容器 BTreeMap ,保存了该逻辑段内的每个虚拟页面的 VPN 和被映射到的物理页帧<br>这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
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
    /// 键值对容器 BTreeMap ,保存了该逻辑段内的每个虚拟页面的 VPN 和被映射到的物理页帧<br>
    /// 这些物理页帧被用来存放实际内存数据而不是作为多级页表中的中间节点
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    /// 描述该逻辑段内的所有虚拟页面映射到物理页帧的方式
    map_type: MapType,
    /// 控制该逻辑段的访问方式，它是页表项标志位 PTEFlags 的一个子集，仅保留 `U` `R` `W` `X` 四个标志位
    map_perm: MapPermission,

    // 决赛补充
    start_va: VirtAddr,
    end_va: VirtAddr,
}

impl MapArea {
    /// ### 根据起始 *(会被下取整)* 和终止 *(会被上取整)* 虚拟地址生成一块逻辑段
    /// - 逻辑段大于等于虚拟地址范围
    pub fn new(start_va: VirtAddr, end_va: VirtAddr, map_type: MapType, map_perm: MapPermission) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            start_va,
            end_va,
        }
    }

    /// ### 从一个逻辑段复制得到一个虚拟地址区间、映射方式和权限控制均相同的逻辑段
    /// 不同的是由于它还没有真正被映射到物理页帧上，所以 data_frames 字段为空
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            start_va: another.start_va,
            end_va: another.end_va,
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
                // println!("map_one");
                let frame = frame_alloc().expect("out of memory");
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

    /// 在多级页表中删除指定vpn对应的映射
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    /// 在多级页表中为逻辑块分配空间
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            // println!("map vpm:0x{:x}",vpn.0);
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

    pub fn copy_data(&mut self, page_table: &mut PageTable, elf_file: Arc<OSInode>, data_start:usize, data_len: usize, page_offset: usize) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut offset: usize = 0;
        let mut page_offset: usize = page_offset;
        let mut current_vpn = self.vpn_range.get_start();
        let mut data_len = data_len;
        // println!("data_len:{}, page_offset:{}",data_len,page_offset);
        loop {
            // println!("current_vpn:0x{:x}, offset:{}",current_vpn.0,offset);
            // println!("data_len.min(PAGE_SIZE): {}",data_len.min(PAGE_SIZE));
            let data = elf_file.read_vec((data_start + offset) as isize, data_len.min(PAGE_SIZE));
            // println!("data:{:?}",data);
            let data_silce = data.as_slice();
            let src = &data_silce[0..data_len.min(PAGE_SIZE - page_offset)];
            let dst = &mut page_table.translate(current_vpn).unwrap().ppn().get_bytes_array()[page_offset..page_offset + src.len()];
            dst.copy_from_slice(src);
            offset += PAGE_SIZE - page_offset;

            page_offset = 0;
            data_len -= src.len();
            if data_len == 0 {
                break;
            }
            current_vpn.step();
        }
    }

    pub fn insert_tracker(&mut self, vpn: VirtPageNum, ppn: PhysPageNum) {
        self.data_frames.insert(vpn, FrameTracker::from_ppn(ppn));
    }
}