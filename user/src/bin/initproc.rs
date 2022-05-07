// 用户初始程序
// user/src/bin/initproc.rs

#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    if fork() == 0 {// 子进程执行 user_shell
        // exec("user_shell\0", &[core::ptr::null::<u8>()]);
        exec("c_usertests\0", &[core::ptr::null::<u8>()]);
    } else {
        loop {      // 父进程等待子进程结束，回收资源
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
    0
}
