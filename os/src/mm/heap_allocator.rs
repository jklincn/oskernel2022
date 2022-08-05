// 堆空间大小为 1310720，即 1.25M

use simple_chunk_allocator::{heap, heap_bitmap, GlobalChunkAllocator, PageAligned};
use crate::config::KERNEL_HEAP_SIZE;

static mut HEAP: PageAligned<[u8; KERNEL_HEAP_SIZE]> = heap!(chunks=5632, chunksize=256);
static mut HEAP_BITMAP: PageAligned<[u8; 704]> = heap_bitmap!(chunks=5632);

#[global_allocator]
pub static ALLOCATOR: GlobalChunkAllocator =
    unsafe { GlobalChunkAllocator::new(HEAP.deref_mut_const(), HEAP_BITMAP.deref_mut_const()) };

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub fn heap_usage(){
    let usage = ALLOCATOR.usage();
    let used = usage / 100.0 * KERNEL_HEAP_SIZE as f32;
    println!("[kernel] heap usage: {}% ({}/{} bytes)",usage as usize,used as usize,KERNEL_HEAP_SIZE);
}