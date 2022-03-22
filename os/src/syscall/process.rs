/// # 进程控制模块
/// `os/src/syscall/process.rs`
/// ## 实现功能
/// ```
/// pub fn sys_exit(exit_code: i32) -> !
/// pub fn sys_yield() -> isize
/// pub fn sys_get_time() -> isize
/// pub fn sys_getpid() -> isize
/// pub fn sys_fork() -> isize
/// pub fn sys_exec(path: *const u8) -> isize
/// pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize
/// ```
//

use crate::loader::get_app_data_by_name;
use crate::mm::{translated_refmut, translated_str};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next,
};
use crate::timer::get_time_ms;
use alloc::sync::Arc;

/// 结束进程运行然后运行下一程序
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// ### 应用主动交出 CPU 所有权进入 Ready 状态并切换到其他应用
/// - 返回值：总是返回 0。
/// - syscall ID：124
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// 获取CPU上电时间（单位：ms）
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

/// 获取当前正在运行程序的 PID
pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

/// ### 当前进程 fork 出来一个子进程。
/// - 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID 。
/// - syscall ID：220
pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // trap_handler 已经将当前进程 Trap 上下文中的 sepc 向后移动了 4 字节，
    // 使得它回到用户态之后，会从发出系统调用的 ecall 指令的下一条指令开始执行

    // 对于子进程，返回值是0
    trap_cx.x[10] = 0;
    // 将 fork 到的进程加入任务调度器
    add_task(new_task);
    // 对于父进程，返回值是子进程的 PID
    new_pid as isize
}

/// ### 将当前进程的地址空间清空并加载一个特定的可执行文件，返回用户态后开始它的执行。
/// - 参数：path 给出了要加载的可执行文件的名字；
/// - 返回值：如果出错的话（如找不到名字相符的可执行文件）则返回 -1，否则不应该返回。
/// - syscall ID：221
pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    // 读取到用户空间的应用程序名称（路径）
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}