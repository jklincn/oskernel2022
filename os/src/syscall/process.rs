/// # 进程控制模块
/// `os/src/syscall/process.rs`
/// ## 实现功能
/// ```
/// pub fn sys_exit(exit_code: i32) -> !
/// pub fn sys_yield() -> isize
/// pub fn sys_get_time() -> isize
/// ```
//

use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::get_time_ms;

/// ### 结束进程运行并将程序返回值打印到终端，然后运行下一程序
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// ### 应用主动交出 CPU 所有权进入 Ready 状态并切换到其他应用
/// - 返回值：总是返回 0。
/// - syscall ID：124
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// ### 获取CPU上电时间（单位：ms）
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
