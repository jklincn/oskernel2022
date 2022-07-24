/// # 堆空间内存动态分配模块
/// `os/src/mm/heap_allocator.rs`
/// ## 实现功能
/// - 在内核空间 .bss 段中开辟了一段空间，用作堆空间，使用伙伴系统堆这块内存进行分配
/// - 实现堆数据结构后才能使用 `alloc` 提供的堆上数据结构
/// ```
/// static HEAP_ALLOCATOR: LockedHeap
/// pub fn init_heap()
/// pub fn handle_alloc_error(layout: core::alloc::Layout) -> !
/// ```
//

use crate::config::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;

// 将 buddy_system_allocator 中提供的 LockedHeap 实例化成一个全局变量
// 并使用 alloc 要求的 #[global_allocator] 语义项进行标记
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// 用于处理动态内存分配失败的情形,直接panic
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}
/// 给全局分配器用于分配的一块内存，位于内核的 .bss 段中
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 在使用任何 alloc 中提供的堆数据结构之前，我们需要先调用 init_heap 函数来给我们的全局分配器一块内存用于分配。
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()// LockedHeap 也是一个被互斥锁 Mutex<T> 保护的类型，
            // 在对它任何进行任何操作之前都要先获取锁以避免其他线程同时对它进行操作导致数据竞争
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
            // 调用 init 方法告知它能够用来分配的空间的起始地址和大小即可
    }
}