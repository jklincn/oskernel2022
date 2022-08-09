use alloc::sync::Arc;
use crate::fs::OSInode;
use core::fmt::{self, Debug, Formatter};
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
    /// |MAP_SHARED|0x01|共享映射，修改对所有进程可见，多进程读写同一个文件需要调用者提供互斥机制|
    /// |MAP_PRIVATE|0x02|私有映射，进程A的修改对进程B不可见的，利用 COW 机制，修改只会存在于内存中，不会同步到外部的磁盘文件上|
    /// |MAP_FIXED|0x10||
    /// |MAP_ANONYMOUS|0x20|匿名映射，初始化全为0的内存空间|
    pub struct MmapFlags: u32 {
        const MAP_FILE = 0;
        const MAP_SHARED= 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
        const MAP_ANONYMOUS = 0x20;
        const MAP_ELF = 0x40;
    }
    // 应用场景：
    // MAP_FILE | MAP_SHARED: 两个进程共同读写一个文本文件
    // MAP_FILE | MAP_PRIVATE: 进程对动态链接库的使用
    // MAP_ANONYMOUS | MAP_SHARED: 作为进程间通信机制的POSIX共享内存(Linux 中共享内存对应tmpfs的一个文件，也可视为共享文件映射)
    // MAP_ANONYMOUS | MAP_PRIVATE: malloc()
}

#[derive(Clone)]
pub struct VMArea {
    // pub vm_start: VirtAddr,
    // pub vm_end: VirtAddr,
    pub vm_start_page: VirtPageNum,
    pub vm_end_page: VirtPageNum,
    pub vm_prot: MmapProts,
    pub vm_flags: MmapFlags,
    // 如果是文件映射，则下面保存文件节点、起始偏移量与该段数据长度
    pub file: Option<Arc<OSInode>>,
    pub offset: usize,
    pub file_len: usize,
}

impl Debug for VMArea {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        println!("\n---------VMArea--------");
        println!("vm_start_page: 0x{:x}",self.vm_start_page.0);
        println!("vm_end_page: 0x{:x}",self.vm_end_page.0);
        println!("vm_prot: {:?}",self.vm_prot);
        println!("vm_flags: {:?}",self.vm_flags);
        println!("file: {:?}",self.file);
        println!("offset: 0x{:x}",self.offset);
        println!("file_len: 0x{:x}",self.file_len);
        f.write_fmt(format_args!("-----------------------"))
    }
}

impl VMArea {
    pub fn new(
        vm_start: VirtAddr,
        vm_end: VirtAddr,
        vm_prot: MmapProts,
        vm_flags: MmapFlags,
        file: Option<Arc<OSInode>>,
        offset: usize,
        file_len: usize,
    ) -> Self {
        let vm_start_page = vm_start.floor();
        let vm_end_page = vm_end.ceil();
        Self {
            // vm_start,
            // vm_end,
            vm_start_page,
            vm_end_page,
            vm_prot,
            vm_flags,
            file,
            offset,
            file_len,
        }
    }
}
