use super::{VirtAddr, UserBuffer, translated_byte_buffer};
use crate::config::PAGE_SIZE;
use crate::fs::File;
use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::fs::Kstat;

bitflags! {
    pub struct MmapProts: usize {
        const PROT_NONE = 0;
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
    /// |MAP_SHARED|0x01|共享映射，多进程间数据共享，修改反应到磁盘实际文件中。|
    /// |MAP_PRIVATE|0x02|私有映射，多进程间数据共享，修改不反应到磁盘实际文件，|
    /// |MAP_FIXED|0x10||
    /// |MAP_ANONYMOUS|0x20|匿名映射，初始化全为0的内存空间|
    pub struct MmapFlags: usize {
        const MAP_FILE = 0;
        const MAP_SHARED= 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
        const MAP_ANONYMOUS = 0x20;
    }
}

/// ### mmap 块管理器
/// - `mmap_start` : 地址空间中mmap区块起始虚地址
/// - `mmap_top` : 地址空间中mmap区块当结束虚地址
/// - `mmap_set` : mmap块 向量
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

    pub fn lazy_map_page(&mut self, page_start: VirtAddr, fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>, token: usize) {
        let offset: usize = self.offset - self.oaddr.0 + page_start.0;
        // println!("[Kernel mmap] map file 0x{:X} = 0x{:X} - 0x{:X} + 0x{:X}", offset, self.offset, self.oaddr.0, page_start.0);
        self.map_file(page_start, PAGE_SIZE, offset, fd_table, token);
    }

    pub fn map_file(&mut self, va_start: VirtAddr, len: usize, offset: usize, fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>, token: usize) -> isize {
        let flags = MmapFlags::from_bits(self.flags).unwrap();
        // println!("[Kernel mmap] map_file: va_strat:0x{:X} flags:{:?}",va_start.0, flags);
        if flags.contains(MmapFlags::MAP_ANONYMOUS)
            && self.fd == -1 
            && offset == 0{
            println!("[map_anonymous_file]");
            return 1;
        }
        
        if self.fd as usize >= fd_table.len() { return -1; }

        if let Some(file) = &fd_table[self.fd as usize] {
            let f = file.clone();
            f.set_offset(offset);
            if !f.readable() { return -1; }
            // println!{"The va_start is 0x{:X}, offset of file is {}", va_start.0, offset};
            let _read_len = f.read(UserBuffer::new(translated_byte_buffer(token, va_start.0 as *const u8, len)));
            // println!{"[kernel mmap] read {} bytes", _read_len};
            // println!("[kernel] {:?}",va_start);
        } else { return -1 };
        return 1;
    }
}