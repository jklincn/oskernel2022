#![no_std]  // 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core（core库不需要操作系统的支持）
#![no_main] 

mod lang_items;

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));
