/// # 任务管理模块
/// `os/src/task/mod.rs`
/// ## 实现功能
/// ```
/// pub struct TaskManager
/// // 全局(唯一)变量任务管理器
/// pub static ref TASK_MANAGER: TaskManager
/// pub fn run_first_task()
/// pub fn suspend_current_and_run_next()
/// pub fn exit_current_and_run_next()
/// ```
//

mod context;// 任务上下文模块
mod switch; // 任务上下文切换模块
#[allow(clippy::module_inception)]
mod task;   // 任务控制块

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// ### 任务管理器
/// 唯一全局变量 `TASK_MANAGER`
pub struct TaskManager {
    /// 应用程序总数，不变的
    num_app: usize,
    /// 会改变，且需要全局访问的数据部分
    /// - `tasks`:任务控制块数组
    /// - `current_task`: CPU 正在执行的应用编号
    inner: UPSafeCell<TaskManagerInner>,
}

/// ### 任务管理器内部需要全局访问的数据
struct TaskManagerInner {
    /// 任务控制块数组
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// CPU 正在执行的应用编号
    current_task: usize,
}

// 初始化`TASK_MAMADER`的全局实例
lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        // 将任务上下文中的寄存器初始化为0
        // 将任务状态初始化为为初始化的状态
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        // 依次对每个任务控制块进行初始化
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        // 创建 TaskManager 实例并返回
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// 运行第一个程序
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // 在此之前，我们应该删除必须手动删除的局部变量 inner
        unsafe {
            // 相当于用0交换下一个程序的任务上下文
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// 将当前程序标记为准备运行
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// 将当前程序标记为已退出
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// 找到 `current_task` 后面第一个状态为 `Ready` 的应用
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
            // 通过编译器参数实现了比较
        // 一般情况下 inner 会在函数退出之后会被自动释放
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            // 获取访问权限，读出当前程序号
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            // 修改任务状态和当前运行的程序号
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            // 获取切换前后任务上下文
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // 在此之前，我们应该删除必须手动删除的局部变量 inner 
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // 回到用户模式
        } else {
            panic!("All applications completed!");
        }
    }
}

/// 运行第一个(编号为0的)程序
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// 将当前程序标记为暂停
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// 将当前程序标记为已退出
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// ### 暂停当前的应用并切换到下个应用
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// ### 退出当前的应用并切换到下个应用
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
