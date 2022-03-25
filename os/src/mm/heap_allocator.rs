use crate::config::KERNEL_HEAP_SIZE; // 0x30_0000 = 3MB

// 使用一个已有的伙伴分配器实现
use buddy_system_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty(); //初始化

// 动态内存分配失败的情况
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

// 堆区域，位于内核的 .bss 段中
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0u8; KERNEL_HEAP_SIZE];

// 初始化堆分配器
pub fn init_heap() {
    // unsafe：访问可变静态变量，Rust 认为存在数据竞争风险
    unsafe {
        // 将空间首地址和空间大小告知分配器
        HEAP_ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}
