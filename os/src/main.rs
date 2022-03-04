// os/src/main.rs

#![no_std]  // 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core（core库不需要操作系统的支持）
#![no_main] // 不使用main函数，而使用汇编代码指定的入口
#![feature(panic_info_message)] // 让panic函数能通过 PanicInfo::message 获取报错信息

#[macro_use]
mod console;// 控制台模块
mod batch;  // 简单的批处理系统
mod lang_items; // 
mod sbi;    // 实现了 RustSBI 通信的相关功能
mod sync;   // 允许在单核处理器上将引用做全局变量使用
mod syscall;// 系统调用模块
mod trap;   // 提供 Trap 管理


use core::arch::global_asm;
global_asm!(include_str!("entry.asm")); // 代码的第一条语句，执行指定的汇编文件，汇编程序再调用Rust实现的内核
global_asm!(include_str!("link_app.S"));// 用于将应用程序的二进制镜像文件作为内核的数据段链接到内核，根据build.rs自动生成

// 通过宏将 rust_main 标记为 #[no_mangle] 以避免编译器对它的名字进行混淆，不然在链接的时候，
// entry.asm 将找不到 main.rs 提供的外部符号 rust_main 从而导致链接失败
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    batch::init();
    batch::run_next_app();
}

/// 初始化内存.bbs区域
fn clear_bss(){
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }     //迭代器与闭包
}
