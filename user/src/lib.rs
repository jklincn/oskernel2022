// user/src/lib.rs
// 用户模式下程序的主文件

#![no_std]
#![feature(linkage)]    // 为了支持软链接操作而加入
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod lang_items;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    // 调用main函数得到一个类型为i32的返回值
    // 最后调用用户库提供的 exit 接口退出应用程序
    // 并将 main 函数的返回值告知批处理系统
    exit(main());   
    panic!("unreachable after sys_exit!");
}

// 我们使用 Rust 的宏将其函数符号 main 标志为弱链接。
// 这样在最后链接的时候，虽然在 lib.rs 和 bin 目录下的某个应用程序
// 都有 main 符号，但由于 lib.rs 中的 main 符号是弱链接，
// 链接器会使用 bin 目录下的应用主逻辑作为 main 。
// 这里我们主要是进行某种程度上的保护，如果在 bin 目录下找不到任何 main ，
// 那么编译也能够通过，但会在运行时报错。
#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

mod syscall;
use syscall::*;

/// ### 打印输出
/// - `fd` : 文件描述符
///     - 1表示标准输出
/// - `buf`: 缓冲区起始地址
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}