/// # 任务控制块
/// `os/src/task/task.rs`
/// ```
/// pub struct TaskControlBlock
/// pub enum TaskStatus
/// ```
//

use super::TaskContext;

#[derive(Copy, Clone)]  // 由编译器实现一些特性

/// ### 任务控制块
pub struct TaskControlBlock {
    /// 任务状态
    pub task_status: TaskStatus,
    /// 任务上下文
    pub task_cx: TaskContext,
}

#[derive(Copy, Clone, PartialEq)]

/// 任务状态枚举
pub enum TaskStatus {
    UnInit, // 未初始化
    Ready,  // 准备运行
    Running,// 正在运行
    Exited, // 已退出
}
