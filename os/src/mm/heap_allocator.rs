// 模块下有两套堆分配器
// 1. simple_chunk_allocator(如交到评测机还需添加本地库依赖)
// 2. linked_list_allocator(正在使用)

// use crate::config::KERNEL_HEAP_SIZE;
// use simple_chunk_allocator::{heap, heap_bitmap, GlobalChunkAllocator, PageAligned};

// static mut HEAP: PageAligned<[u8; KERNEL_HEAP_SIZE]> = heap!(chunks = 5632, chunksize = 256);
// static mut HEAP_BITMAP: PageAligned<[u8; 704]> = heap_bitmap!(chunks = 5632);

// #[global_allocator]
// pub static ALLOCATOR: GlobalChunkAllocator = unsafe { GlobalChunkAllocator::new(HEAP.deref_mut_const(), HEAP_BITMAP.deref_mut_const()) };

// pub fn init_heap() {}

// pub fn heap_usage() {
//     let usage = ALLOCATOR.usage();
//     let used = usage / 100.0 * KERNEL_HEAP_SIZE as f32;
//     println!(
//         "[kernel] heap usage: {:.2}% ({}/{} bytes)",
//         usage, used as usize, KERNEL_HEAP_SIZE
//     );
// }

use linked_list_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    unsafe {
        ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
        // println!("bottom:0x{:x}",ALLOCATOR.lock().bottom());
        // println!("top:0x{:x}",ALLOCATOR.lock().top());
    }
}

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    heap_usage();
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub fn heap_usage(){
    let used = ALLOCATOR.lock().used();
    let total_size = ALLOCATOR.lock().size();
    let usage = used as f64 / total_size as f64 * 100.0;
    println!("[kernel] heap usage: {:.2}% ({}/{} bytes)", usage, used as usize,total_size);
}
