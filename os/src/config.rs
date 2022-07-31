/// # 参数库
/// `os/src/config.rs`
/// 
/// 定义了一些参数
//

pub const USER_STACK_SIZE:      usize = 4096 * 20;
pub const KERNEL_STACK_SIZE:    usize = 4096 * 2;

pub const USER_HEAP_SIZE: usize = 4096 * 20;

// 由于决赛最终将评测转移到了 qemu 平台中,因此不存在内存限制
pub const KERNEL_HEAP_SIZE:     usize = 4096 * 256 * 30;  // 30MB

/// 指定内存终止物理地址，内存大小为6MiB（左闭右开）(8M有大坑，会随机卡死)
#[cfg(feature = "board_k210")]
pub const MEMORY_END:           usize = 0x80600000;
#[cfg(not(any(feature = "board_k210")))]
pub const MEMORY_END:           usize = 0x88000000; // 128MB

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
