/// # 任务上下文切换模块
/// `os/src/task/switch.rs`
/// ## 实现功能
/// ```
/// pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
/// ```
//

use super::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

extern "C" {
    // 将汇编代码中的全局符号 __switch 解释为一个 Rust 函数
    // 切换任务上下文
    // current_task_cx_ptr 当前任务上下文指针
    // next_task_cx_ptr    即将被切换到的任务上下文指针
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
