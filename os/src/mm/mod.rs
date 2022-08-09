/// # 内存管理模块
/// `os/src/mm/mod.rs`
/// ## 实现功能
/// ```
/// pub fn init()
/// ```
//

mod address;        // 地址数据类型
mod frame_allocator;// 物理页帧管理器
mod heap_allocator; // 堆空间内存动态分配模块
mod memory_set;     // 地址空间模块
mod page_table;     // 页表
mod vma;            // 虚拟内存地址映射空间

use address::VPNRange;
pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker,frame_usage};
pub use memory_set::{kernel_token, MapPermission, MemorySet, KERNEL_SPACE};
use page_table::PTEFlags;
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};
pub use vma::*;
pub use heap_allocator::heap_usage;

/// 内存管理子系统的初始化
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    // 从这一刻开始 SV39 分页模式就被启用了
    KERNEL_SPACE.lock().activate();
}

pub fn memory_usage(){
    println!("---------------------Memory usage---------------------");
    frame_allocator::frame_usage();
    heap_allocator::heap_usage();
    println!("------------------------------------------------------");
}
