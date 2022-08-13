/// # 页表
/// `os/src/mm/page_table.rs`
/// ## 实现功能
/// ```
/// pub struct PTEFlags: u8
/// pub struct PageTableEntry
/// pub struct PageTable
///
/// pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]>
/// ```
//

use crate::config::PAGE_SIZE;

use super::{frame_alloc, FrameTracker};
use super::address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use _core::mem::size_of;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

// 可以将一个 u8 封装成一个标志位的集合类型，支持一些常见的集合运算
bitflags! {
    /// ### 页表项标志位
    /// |标志位|描述|
    /// |--|--|
    /// |`V(Valid)`|仅当位 V 为 1 时，页表项才是合法的；
    /// |`R(Read)` `W(Write)` `X(eXecute)`|分别控制索引到这个页表项的对应虚拟页面是否允许读/写/执行；
    /// |`U(User)`|控制索引到这个页表项的对应虚拟页面是否在 CPU 处于 U 特权级的情况下是否被允许访问；
    /// |`G`|暂且不理会；
    /// |`A(Accessed)`|处理器记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被访问过；
    /// |`D(Dirty)`|处理器记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被修改过
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

/// ### 页表项
/// - `bits`
/// ```
/// PageTableEntry::new(ppn: PhysPageNum, flags: PTEFlags) -> Self
/// PageTableEntry::empty() -> Self
/// PageTableEntry::ppn(&self) -> PhysPageNum
/// PageTableEntry::flags(&self) -> PTEFlags
/// PageTableEntry::is_valid(&self) -> bool
/// PageTableEntry::readable(&self) -> bool
/// PageTableEntry::writable(&self) -> bool
/// PageTableEntry::executable(&self) -> bool
/// PageTableEntry::set_pte_flags(&mut self, flags: usize)
/// ```
#[derive(Copy, Clone)]
// 让编译器自动为 PageTableEntry 实现 Copy/Clone Trait，来让这个类型以值语义赋值/传参的时候不会发生所有权转移，而是拷贝一份新的副本
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    /// 从一个物理页号 `PhysPageNum` 和一个页表项标志位 `PTEFlags` 生成一个页表项 `PageTableEntry` 实例
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// 将页表项清零
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// 从页表项读取物理页号
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// 验证页表项是否合法（V标志位是否为1）
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// 验证页表项是否可读（R标志位是否为1）
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// 验证页表项是否可写（W标志位是否为1）
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// 验证页表项是否可执行（X标志位是否为1）
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
    // only X+W+R can be set
    pub fn set_pte_flags(&mut self, flags: usize) {
        self.bits = (self.bits & !(0b1110 as usize)) | (flags & (0b1110 as usize));
    }

    pub fn set_flags(&mut self, flags: PTEFlags) {
        let new_flags: u8 = flags.bits().clone();
        self.bits = (self.bits & 0xFFFF_FFFF_FFFF_FF00) | (new_flags as usize);
    }

    pub fn set_cow(&mut self) {
        (*self).bits = self.bits | (1 << 9);
    }

    pub fn reset_cow(&mut self) {
        (*self).bits = self.bits & !(1 << 9);
    }

    pub fn is_cow(&self) -> bool {
        self.bits & (1 << 9) != 0
    }
}

/// ### SV39多级页表
/// - `root_ppn`:根节点的物理页号,作为页表唯一的区分标志
/// - `frames`:以 FrameTracker 的形式保存了页表所有的节点（包括根节点）所在的物理页帧
///
/// 一个地址空间对应一个页表
///
/// ```
/// PageTable::map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags)
/// PageTable::unmap(&mut self, vpn: VirtPageNum)
/// PageTable::translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry>
/// PageTable::token(&self) -> usize
/// ```
#[derive(Debug)]
pub struct PageTable {
    /// 根节点的物理页号,作为页表唯一的区分标志
    root_ppn: PhysPageNum,
    /// 以 FrameTracker 的形式保存了页表所有的节点（包括根节点）所在的物理页帧
    /// 用以延长物理页帧的生命周期
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// 新建一个 `PageTable`
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame], // 将新获取到的物理页帧存入向量
        }
    }

    /// 临时通过 `satp` 获取对应的多级页表
    pub fn from_token(satp: usize) -> Self {
        Self {
            // 取satp的前44位作为物理页号
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            // 不需要重新生成节点，节点已经在原始多级页表中存在，同时存在在内存中
            frames: Vec::new(),
        }
    }

    /// 根据vpn查找对应页表项，如果在查找过程中发现无效页表则新建页表
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        // 当前节点的物理页号，最开始指向多级页表的根节点
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            // 通过 get_pte_array 将取出当前节点的页表项数组，并根据当前级页索引找到对应的页表项
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                // 找到第三级页表，这个页表项的可变引用
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                // 发现页表项是无效的状态
                // 获取一个物理页帧
                let frame = frame_alloc().unwrap();
                // 用获取到的物理页帧生成新的页表项
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                // 将生成的页表项存入页表
                self.frames.push(frame);
            }
            // 切换到下一级页表（物理页帧）
            ppn = pte.ppn();
        }
        result
    }

    /// 根据vpn查找对应页表项，如果在查找过程中发现无效页表则直接返回 None 即查找失败
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    /// ### 建立一个虚拟页号到物理页号的映射
    /// 根据VPN找到第三级页表中的对应项，将 `PPN` 和 `flags` 写入到页表项
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        // 断言，保证新获取到的PTE是无效的（不是已分配的）
        assert!(!pte.is_valid(), "{:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// ### 删除一个虚拟页号到物理页号的映射
    /// 只需根据虚拟页号找到页表项，然后修改或者直接清空其内容即可
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "{:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// ### 根据 vpn 查找页表项
    /// 调用 `find_pte` 来实现，如果能够找到页表项，那么它会将页表项拷贝一份并返回，否则就返回一个 `None`
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    /// 在当前多级页表中将虚拟地址转换为物理地址
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: PhysAddr = pte.ppn().into();
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    /// 按照 satp CSR 格式要求 构造一个无符号 64 位无符号整数，使得其分页模式为 SV39 ，且将当前多级页表的根节点所在的物理页号填充进去
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }

    // only X+W+R can be set
    // return -1 if find no such pte
    pub fn set_pte_flags(&mut self, vpn: VirtPageNum, flags: usize) -> isize {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                pte.set_pte_flags(flags);
                break;
            }
            if !pte.is_valid() {
                return -1;
            }
            ppn = pte.ppn();
        }
        0
    }

    pub fn set_cow(&mut self, vpn: VirtPageNum) {
        self.find_pte_create(vpn).unwrap().set_cow();
    }

    pub fn reset_cow(&mut self, vpn: VirtPageNum) {
        self.find_pte_create(vpn).unwrap().reset_cow();
    }

    pub fn set_flags(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        self.find_pte_create(vpn).unwrap().set_flags(flags);
    }

    pub fn remap_cow(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, former_ppn: PhysPageNum) {
        let pte = self.find_pte_create(vpn).unwrap();
        *pte = PageTableEntry::new(ppn, pte.flags() | PTEFlags::W);
        pte.set_cow();
        ppn.get_bytes_array().copy_from_slice(former_ppn.get_bytes_array());
    }
}

/// ### 以向量的形式返回一组可以在内存空间中直接访问的字节数组切片
/// |参数|描述|
/// |--|--|
/// |`token`|某个应用地址空间的 token|
/// |`ptr`|应用地址空间中的一段缓冲区的起始地址
/// |`len`|应用地址空间中的一段缓冲区的长度
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).expect("[kernel] translated_byte_buffer: page not mapped!").ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// ### 从内核地址空间之外的某个应用的用户态地址空间中拿到一个字符串
/// 针对应用的字符串中字符的用户态虚拟地址，查页表，找到对应的内核虚拟地址，逐字节地构造字符串，直到发现一个 \0 为止
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table.translate_va(VirtAddr::from(va)).unwrap().get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

/// 根据 多级页表token (satp) 和 虚拟地址 获取大小为 T 的空间的不可变切片
pub fn translated_ref<T>(token: usize, ptr: *const T) -> &'static T {
    let offset = ptr as usize % PAGE_SIZE;
    assert!(PAGE_SIZE - offset >= size_of::<T>(), "cross-page access");
    let page_table = PageTable::from_token(token);
    page_table.translate_va(VirtAddr::from(ptr as usize)).unwrap().get_ref()
}

/// 根据 多级页表token (satp) 和 虚拟地址 获取大小为 T 的空间的切片
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let offset = ptr as usize % PAGE_SIZE;
    assert!(PAGE_SIZE - offset >= size_of::<T>(), "cross-page access");
    //println!("into translated_refmut!");
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table.translate_va(VirtAddr::from(va)).unwrap().get_mut()
}

/// ### 应用地址空间中的一段缓冲区（即内存）的抽象
/// - `buffers`：位于应用地址空间中，内核无法直接通过用户地址空间的虚拟地址来访问，因此需要进行封装
#[derive(Debug)]
pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub fn empty() -> Self {
        Self { buffers: Vec::new() }
    }

    /// 使用 `buffer` 创建一个新的缓冲区实例
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }

    // 将一个Buffer的数据写入UserBuffer，返回写入长度
    pub fn write(&mut self, buff: &[u8]) -> usize {
        let len = self.len().min(buff.len());
        let mut current = 0;
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                (*sub_buff)[j] = buff[current];
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        return len;
    }

    pub fn write_at(&mut self, offset: usize, buff: &[u8]) -> isize {
        let len = buff.len();
        if offset + len > self.len() {
            panic!();
        }
        let mut head = 0; // offset of slice in UBuffer
        let mut current = 0; // current offset of buff

        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            if head + sblen < offset {
                head += sblen;
                continue;
            } else if head < offset { 
                for j in (offset - head)..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            } else {
                //head + sblen > offset and head > offset
                for j in 0..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            }
            head += sblen;
        }
        0
    }

    // 将UserBuffer的数据读入一个Buffer，返回读取长度
    pub fn read(&self, buff: &mut [u8]) -> usize {
        let len = self.len().min(buff.len());
        let mut current = 0;
        for sub_buff in self.buffers.iter() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                buff[current] = (*sub_buff)[j];
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        return len;
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
