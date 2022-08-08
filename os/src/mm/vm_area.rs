use alloc::sync::Arc;

use crate::fs::OSInode;

use super::{VirtAddr, VirtPageNum};

bitflags! {
    pub struct MmapProts: u32 {
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
    pub struct MmapFlags: u32 {
        const MAP_FILE = 0;
        const MAP_SHARED= 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
        const MAP_ANONYMOUS = 0x20;
    }
}

#[derive(Debug,Clone)]
pub struct VMArea {
    pub vm_start: VirtAddr,
    pub vm_end: VirtAddr,
    pub vm_start_page: VirtPageNum,
    pub vm_end_page: VirtPageNum,
    pub vm_prot: MmapProts,
    pub vm_flags: MmapFlags,
    // 如果是文件映射，则下面保存文件节点、起始偏移量与该段数据长度
    pub file:isize,
    pub offset: usize,
    pub file_len:usize,
    // ELF文件
    pub elf: Option<Arc<OSInode>>,
}

impl VMArea {
    pub fn new(vm_start: VirtAddr, vm_end: VirtAddr, vm_prot: MmapProts, vm_flags: MmapFlags,file:isize, offset: usize,file_len:usize, elf: Option<Arc<OSInode>>) -> Self {
        let vm_start_page = vm_start.floor();
        let vm_end_page = vm_start.ceil();
        Self {
            vm_start,
            vm_end,
            vm_start_page,
            vm_end_page,
            vm_prot,
            vm_flags,
            file,
            offset,
            file_len,
            elf,
        }
    }
}
