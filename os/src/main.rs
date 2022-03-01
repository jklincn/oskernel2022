#![no_std]  // 告诉 Rust 编译器不使用 Rust 标准库 std 转而使用核心库 core（core库不需要操作系统的支持）
#![no_main] 
#[cfg(not(test))]   //用以解决lang items重复的问题（不加vscode会报错，能过编译）

mod lang_items;

