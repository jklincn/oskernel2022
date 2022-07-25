use crate::config::KERNEL_HEAP_SIZE;
// use buddy_system_allocator::LockedHeap;

use simple_chunk_allocator::{heap, heap_bitmap, GlobalChunkAllocator, PageAligned};

static mut HEAP: PageAligned<[u8; KERNEL_HEAP_SIZE]>= heap!(chunks=12288,chunksize=256);
static mut HEAP_BITMAP: PageAligned<[u8; 1536]>= heap_bitmap!(chunks=12288);

#[global_allocator]
static ALLOCATOR: GlobalChunkAllocator =
    unsafe { GlobalChunkAllocator::new(HEAP.deref_mut_const(), HEAP_BITMAP.deref_mut_const()) };

/// 在使用任何 alloc 中提供的堆数据结构之前，我们需要先调用 init_heap 函数来给我们的全局分配器一块内存用于分配。
// pub fn init_heap() {
//     unsafe {
//         HEAP_ALLOCATOR
//             .lock() // LockedHeap 也是一个被互斥锁 Mutex<T> 保护的类型，
//             // 在对它任何进行任何操作之前都要先获取锁以避免其他线程同时对它进行操作导致数据竞争
//             .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
//         // 调用 init 方法告知它能够用来分配的空间的起始地址和大小即可
//     }
// }

pub fn init_heap() {
    // unsafe {
    //     HEAP_ALLOCATOR.new(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    // }
}

pub fn heap_allocator_stats() {
    // println!("user requests: {}", HEAP_ALLOCATOR.lock().stats_alloc_user());
    // println!("actually allocated: {}",HEAP_ALLOCATOR.lock().stats_alloc_actual());
    // println!("total: {}", HEAP_ALLOCATOR.lock().stats_total_bytes());
    println!("heap usage: {}%",ALLOCATOR.usage());
}

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    heap_allocator_stats();
    panic!("Heap allocation error, layout = {:?}", layout);
}