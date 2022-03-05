/// # 进程控制模块
/// `os/src/syscall/process.rs`
/// ## 实现功能
/// ```
/// pub fn sys_exit(exit_code: i32) -> !
/// ```
//

use crate::batch::run_next_app;

/// 结束进程运行并将程序返回值打印到终端，然后运行下一程序
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    run_next_app()
}
