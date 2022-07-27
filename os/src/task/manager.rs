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
use alloc::collections::{BTreeMap, VecDeque};
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
    #[allow(unused)]
    pub fn list_alltask(&self){
        let read_queue = self.ready_queue.clone();
        for i in read_queue{
            println!("pid:{}",(*i).pid.0);
        }
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
    pub static ref PID2TCB: UPSafeCell<BTreeMap<usize, Arc<TaskControlBlock>>> =
        unsafe { UPSafeCell::new(BTreeMap::new()) };
}

/// 将一个任务加入到全局 `FIFO 任务管理器` `TASK_MANAGER` 就绪队列的队尾
pub fn add_task(task: Arc<TaskControlBlock>) {
    PID2TCB
        .exclusive_access()
        .insert(task.getpid(), Arc::clone(&task));
    TASK_MANAGER.exclusive_access().add(task);
}

/// 从全局变量 `TASK_MANAGER` 就绪队列的队头中取出一个任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

/// 通过PID获取对应的进程控制块
pub fn pid2task(pid: usize) -> Option<Arc<TaskControlBlock>> {
    let map = PID2TCB.exclusive_access();
    map.get(&pid).map(Arc::clone)
}

pub fn remove_from_pid2task(pid: usize) {
    let mut map = PID2TCB.exclusive_access();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid);
    }
}
