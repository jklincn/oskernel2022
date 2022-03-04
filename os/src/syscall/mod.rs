/// # 系统调用模块
/// `os/src/syscall/mod.rs`
/// ## 实现功能
/// ```
/// pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize
/// ```
//

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

mod fs;         // 文件读写模块
mod process;    // 进程控制模块

use fs::*;
use process::*;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
