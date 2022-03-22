/// # 任务管理器
/// `os/src/task/manager.rs`
/// ```
/// pub struct TaskManager
/// pub static ref TASK_MANAGER: UPSafeCell<TaskManager>
/// 
/// pub fn add_task(task: Arc<TaskControlBlock>)
/// pub fn fetch_task() -> Option<Arc<TaskControlBlock>>
/// ```
//

use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

/// ### FIFO 任务管理器
/// `ready_queue` 就绪进程的进程控制块队列
/// ```
/// TaskManager::new() -> Self
/// TaskManager::add(&mut self, task: Arc<TaskControlBlock>)
/// TaskManager::fetch(&mut self) -> Option<Arc<TaskControlBlock>>
/// ```
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// 将一个任务加入队尾
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// 从队头中取出一个任务
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// 将一个任务加入到全局 `FIFO 任务管理器` `TASK_MANAGER` 就绪队列的队尾
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// 从全局变量 `TASK_MANAGER` 就绪队列的队头中取出一个任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
