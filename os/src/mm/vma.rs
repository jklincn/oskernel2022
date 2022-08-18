use super::{UserBuffer, translated_byte_buffer};
use super::address::VirtAddr;
use crate::config::PAGE_SIZE;
use crate::fs::File;
use alloc::vec::Vec;
use alloc::sync::Arc;
// use core::fmt::{self, Debug, Formatter};

bitflags! {
    pub struct MmapProts: usize {
        const PROT_NONE = 0;  // 不可读不可写不可执行，用于实现防范攻击的guard page等
        const PROT_READ = 1;
        const PROT_WRITE = 1 << 1;
        const PROT_EXEC  = 1 << 2;
        const PROT_GROWSDOWN = 0x01000000;
        const PROT_GROWSUP = 0x02000000;
    }
}

bitflags! {
    /// |名称|值|映射方式|
    /// |--|--|--|
    /// |MAP_FILE|0|文件映射，使用文件内容初始化内存|
    /// |MAP_SHARED|0x01|共享映射，修改对所有进程可见，多进程读写同一个文件需要调用者提供互斥机制|
    /// |MAP_PRIVATE|0x02|私有映射，进程A的修改对进程B不可见的，利用 COW 机制，修改只会存在于内存中，不会同步到外部的磁盘文件上|
    /// |MAP_FIXED|0x10|| 将mmap空间放在addr指定的内存地址上，若与现有映射页面重叠，则丢弃重叠部分。如果指定的地址不能使用，mmap将失败。
    /// |MAP_ANONYMOUS|0x20|匿名映射，初始化全为0的内存空间|

    pub struct MmapFlags: usize {
        const MAP_FILE = 0;
        const MAP_SHARED= 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
        const MAP_ANONYMOUS = 0x20;
    }

    // 应用场景：
    // MAP_FILE | MAP_SHARED: 两个进程共同读写一个文本文件
    // MAP_FILE | MAP_PRIVATE: 进程对动态链接库的使用
    // MAP_ANONYMOUS | MAP_SHARED: 作为进程间通信机制的POSIX共享内存(Linux 中共享内存对应tmpfs的一个文件，也可视为共享文件映射)
    // MAP_ANONYMOUS | MAP_PRIVATE: 常见的是 malloc()

}

/// ### mmap 块管理器
/// - `mmap_start` : 地址空间中mmap区块起始虚地址
/// - `mmap_top` : 地址空间中mmap区块当结束虚地址
/// - `mmap_set` : mmap块 向量
#[derive(Clone,Debug)]
pub struct MmapArea {
    pub mmap_start: VirtAddr,
    pub mmap_top: VirtAddr,
    pub mmap_set: Vec<MmapSpace>,
}

impl MmapArea {
    pub fn new( mmap_start: VirtAddr, mmap_top: VirtAddr ) -> Self {
        Self {
            mmap_start,
            mmap_top,
            mmap_set: Vec::new(),
        }
    }

    pub fn get_mmap_top(&mut self) -> VirtAddr { self.mmap_top }

    pub fn lazy_map_page(&mut self, va: VirtAddr, fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>, token: usize) {
        for mmap_space in self.mmap_set.iter_mut() {
            if va.0 >= mmap_space.oaddr.0 && va.0 < mmap_space.oaddr.0 + mmap_space.length {
                mmap_space.lazy_map_page(va, fd_table, token);
                return 
            }
        }
    }

    pub fn push(&mut self, start: usize, len: usize, prot: usize, flags: usize,
                fd: isize, offset: usize, _fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>, _token: usize) -> usize {
        
        let start_addr = start.into();

        let mmap_space = MmapSpace::new(start_addr, len, prot, flags, 0, fd, offset);
        // mmap_space.map_file(start_addr, PAGE_SIZE, offset, _fd_table, _token);

        self.mmap_set.push(mmap_space);

        // update mmap_top
        if self.mmap_top == start_addr{
            self.mmap_top = (start_addr.0 + len).into();
        }

        start_addr.0
    }

    pub fn remove(&mut self, start: usize, len: usize) -> isize {
        let pair = self.mmap_set.iter().enumerate()
            .find(|(_, p)| { p.oaddr.0 == start });
        if let Some((idx, _)) = pair {
            if self.mmap_top == VirtAddr::from(start + len) {
                self.mmap_top = VirtAddr::from(start);
            }
            self.mmap_set.remove(idx);
            0
        } else {
            panic!{"No matched Mmap Space!"}
        }
    }

    #[allow(unused)]
    pub fn debug_show(&self) {
        println!("------------------MmapArea Layout-------------------");
        println!("MmapArea: mmap_start: 0x{:x}  mmap_top: 0x{:x}", self.mmap_start.0, self.mmap_top.0);
        for mmapspace in &self.mmap_set {
            mmapspace.debug_show();
        }
        println!("----------------------------------------------------");
    }

    pub fn reduce_mmap_range(&mut self, addr:usize, len:usize) {
        for space in self.mmap_set.iter_mut() {
            // 实际上不止这一种情况，todo
            if addr == space.oaddr.0{
                space.oaddr = VirtAddr::from(addr + len);
                space.length = space.length - len;
                if self.mmap_top.0 == space.oaddr.0 + space.length + len {
                    self.mmap_top.0 -= len;
                }
                return;
            }
        }
    }

}

/// ### mmap 块
/// 用于记录 mmap 空间信息，mmap数据并不存放在此
/// 
/// |成员变量|含义|
/// |--|--|
/// |`oaddr`|mmap 空间起始虚拟地址|
/// |`length`|mmap 空间长度|
/// |`valid`|mmap 空间有效性|
/// |`prot`|mmap 空间权限|
/// |`flags`|映射方式|
/// |`fd`|文件描述符|
/// |`offset`|映射文件偏移地址|
/// 
/// - 成员函数
///     ```
///     pub fn new()
///     pub fn lazy_map_page()
///     ```
#[derive(Clone, Copy,Debug)]
pub struct MmapSpace {
    // pub addr: VirtAddr,
    pub oaddr: VirtAddr,
    pub valid: usize,
    pub length: usize,
    pub prot: usize,
    pub flags: usize,
    pub fd: isize,
    pub offset: usize,
}

impl MmapSpace{
    pub fn new(
        oaddr: VirtAddr,
        length: usize,
        prot: usize,
        flags: usize,
        valid: usize,
        fd: isize,
        offset: usize,
    ) -> Self {
        Self {oaddr, length, prot, flags, valid, fd, offset}
    }

    pub fn new_len(&mut self, len:usize) {
        self.length = len;
    }

    pub fn lazy_map_page(&mut self, page_start: VirtAddr, fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>, token: usize) {
        let offset: usize = self.offset - self.oaddr.0 + page_start.0;
        // println!("[Kernel mmap] map_file 0x{:X} = 0x{:X} - 0x{:X} + 0x{:X}", offset, self.offset, self.oaddr.0, page_start.0);
        self.map_file(page_start, PAGE_SIZE, offset, fd_table, token);
    }

    pub fn map_file(&mut self, va_start: VirtAddr, len: usize, offset: usize, fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>, token: usize) -> isize {
        let flags = MmapFlags::from_bits(self.flags).unwrap();
        // println!("[Kernel mmap] map_file: va_strat:0x{:X} flags:{:?}",va_start.0, flags);
        if flags.contains(MmapFlags::MAP_ANONYMOUS)
            && self.fd == -1 
            && offset == 0{
            // println!("[map_anonymous_file]");
            return 1;
        }

        // println!("[Kernel mmap] fd_table.length() {}", fd_table.len());
        // println!("[Kernel mmap] fd {}", self.fd);
        
        if self.fd as usize >= fd_table.len() { return -1; }

        if let Some(file) = &fd_table[self.fd as usize] {
            let f = file.clone();
            f.set_offset(offset);
            if !f.readable() { return -1; }
            // println!{"The va_start is 0x{:X}, offset of file is {}", va_start.0, offset};
            let _read_len = f.read(UserBuffer::new(translated_byte_buffer(token, va_start.0 as *const u8, len)));
            // println!{"[kernel map_file] read {} bytes", _read_len};
            // println!("[kernel] {:?}",va_start);
        } else { return -1 };
        return 1;
    }

    pub fn debug_show(&self) {
        println!("MmapSpace: {:x} ", self.oaddr.0);
    }
}