// os/src/main.rs

#![no_std] // 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core（core库不需要操作系统的支持）
#![no_main] // 不使用main函数，而使用汇编代码指定的入口
#![feature(panic_info_message)] // 让panic函数能通过 PanicInfo::message 获取报错信息
#![feature(alloc_error_handler)] // 用于处理动态内存分配失败的情形

// Simple Chunk Allocator needs
#![feature(const_mut_refs)]
#![feature(allocator_api)]

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[cfg(feature = "board_k210")]
#[path = "boards/k210.rs"]
mod board; // 与硬件板相关的参数
#[cfg(not(any(feature = "board_k210")))]
#[path = "boards/qemu.rs"]
mod board; // 与虚拟机相关的参数
#[macro_use]
mod console; // 控制台模块
mod config; // 参数库
mod drivers; // 设备驱动层
mod fs; // 内核文件系统接口
mod lang_items; // Rust语言相关参数
mod mm; // 内存空间模块
mod sbi; // 实现了 RustSBI 通信的相关功能
// mod sync; // 允许在单核处理器上将引用做全局变量使用
mod syscall; // 系统调用模块
mod task; // 任务管理模块
mod timer; // 时间片模块
mod trap; // 提供 Trap 管理

use core::arch::asm;
use core::arch::global_asm;
use riscv::register::sstatus::{set_fs, FS};

use crate::mm::memory_usage;
global_asm!(include_str!("entry.asm")); // 代码的第一条语句，执行指定的汇编文件，汇编程序再调用Rust实现的内核
global_asm!(include_str!("buildin_app.S")); // 将 c_usertests 程序放入内核区内存空间

pub fn id() -> u64 {
    let cpu_id;
    unsafe {
        asm!("mv {},tp" ,out(reg) cpu_id);
    }
    cpu_id
}

// 通过宏将 rust_main 标记为 #[no_mangle] 以避免编译器对它的名字进行混淆，不然在链接的时候，
// entry.asm 将找不到 main.rs 提供的外部符号 rust_main 从而导致链接失败
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    if id() == 0 {
        unsafe {
            set_fs(FS::Dirty);
        }
        mm::init();
        trap::init();
        trap::enable_timer_interrupt();
        timer::set_next_trigger();
        fs::list_apps();
        task::add_initproc();
        println!("[kernel] Initialization succeeded");
        memory_usage();
        task::run_tasks();
        panic!("Unreachable in rust_main!");
    } else {
        loop {}
    }
}

/// 初始化内存.bbs区域
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize).fill(0);
    }
}
