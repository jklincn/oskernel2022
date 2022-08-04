// 堆空间大小为 1310720，即 1.25M

use simple_chunk_allocator::{heap, heap_bitmap, GlobalChunkAllocator, PageAligned};

static mut HEAP: PageAligned<[u8; 1310720]> = heap!(chunks=5120, chunksize=256);
static mut HEAP_BITMAP: PageAligned<[u8; 640]> = heap_bitmap!(chunks=5120);

#[global_allocator]
pub static ALLOCATOR: GlobalChunkAllocator =
    unsafe { GlobalChunkAllocator::new(HEAP.deref_mut_const(), HEAP_BITMAP.deref_mut_const()) };

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub fn heap_usage(){
    println!("heap usage: {}%",ALLOCATOR.usage());
}