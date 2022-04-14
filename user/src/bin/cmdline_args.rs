#![no_std]
#![no_main]

/// # 命令行参数测试程序
/// `user/src/bin/cmdline_args.rs`
/// 此程序将打印命令行参数个数和每个参数
//

extern crate alloc;

#[macro_use]
extern crate user_lib;

/// 打印命令行参数个数和每个参数
#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    println!("argc = {}", argc);
    for (i, arg) in argv.iter().enumerate() {
        println!("argv[{}] = {}", i, arg);
    }
    0
}
