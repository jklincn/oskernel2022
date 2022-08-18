use linked_list_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    unsafe {
        ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
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
