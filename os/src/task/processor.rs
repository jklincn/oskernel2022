/// # 处理器管理模块
/// `os/src/task/processor.rs`
/// ```
/// pub struct Processor
/// pub static ref PROCESSOR: UPSafeCell<Processor>
/// 
/// pub fn run_tasks()
/// pub fn take_current_task() -> Option<Arc<TaskControlBlock>>
/// pub fn current_task() -> Option<Arc<TaskControlBlock>>
/// pub fn current_user_token() -> usize
/// pub fn current_trap_cx() -> &'static mut TrapContext
/// pub fn schedule(switched_task_cx_ptr: *mut TaskContext)
/// ```
//

use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;

/// ### 处理器管理
/// |成员变量|描述|
/// |--|--|
/// |`current`|当前处理器上正在执行的任务|
/// |`idle_task_cx`|当前处理器上的 idle 控制流的任务上下文|
/// ```
/// Processor::new() -> Self
/// Processor::take_current(&mut self) -> Option<Arc<TaskControlBlock>>
/// Processor::current(&self) -> Option<Arc<TaskControlBlock>>
/// ```
pub struct Processor {  /// 当前处理器上正在执行的任务
    current: Option<Arc<TaskControlBlock>>,
    /// 当前处理器上的 idle 控制流的任务上下文
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    /// 取出当前正在执行的任务
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    /// 返回当前执行的任务的一份拷贝
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    /// - Processor 是描述 CPU执行状态 的数据结构。
    /// - 在单核CPU环境下，我们仅创建单个 Processor 的全局实例 PROCESSOR
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

/// 进入 idle 控制流，它运行在这个 CPU 核的启动栈上，
/// 功能是循环调用 fetch_task 直到顺利从任务管理器中取出一个任务，随后便准备通过任务切换的方式来执行
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        }
    }
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行的任务
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的任务控制块的引用计数的一份拷贝
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的用户地址空间 token
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

/// 换到 idle 控制流并开启新一轮的任务调度
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
