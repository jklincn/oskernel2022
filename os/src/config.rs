/// # 参数库
/// `os/src/config.rs`
/// 
/// 定义了一些参数
//

pub const USER_STACK_SIZE:      usize = 4096 * 10;
pub const KERNEL_STACK_SIZE:    usize = 4096 * 4;

pub const USER_HEAP_SIZE: usize = 4096 * 10;

/// 指定内存终止物理地址，内存大小为6MiB（左闭右开）(8M有大坑，会随机卡死)
#[cfg(feature = "board_k210")]
pub const MEMORY_END:           usize = 0x80600000;
#[cfg(not(any(feature = "board_k210")))]
pub const MEMORY_END:           usize = 0x807E0000;

/// 页面大小：4KiB
pub const PAGE_SIZE:            usize = 0x1000;
/// 页内偏移：12bit
pub const PAGE_SIZE_BITS:       usize = 0xc;

/// 跳板虚拟内存中的起始地址，虚拟内存最高页
pub const TRAMPOLINE:           usize = usize::MAX - PAGE_SIZE + 1;
/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT:         usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MMIO};

pub const MMAP_BASE: usize = 0x60000000;
