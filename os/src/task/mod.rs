/// # 任务管理模块
/// `os/src/task/mod.rs`
/// ## 实现功能
/// ```
/// pub fn suspend_current_and_run_next()
/// pub fn exit_current_and_run_next()
/// pub fn add_initproc()
/// pub fn check_signals_of_current()
/// pub fn current_add_signal()
/// ```
//

mod aux;
mod context; // 任务上下文模块
mod info; // 系统信息模块
mod manager; // 进程管理器
mod pid; // 进程标识符模块
mod processor; // 处理器管理模块
mod resource;
mod signal; // 进程状态标志
mod switch; // 任务上下文切换模块
#[allow(clippy::module_inception)]
mod task; // 进程控制块

use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
use manager::fetch_task;
use manager::remove_from_pid2task;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use aux::*;
pub use context::TaskContext;
pub use info::{CloneFlags, RUsage, Utsname, UTSNAME};
pub use manager::{add_task, pid2task, debug_show_ready_queue};
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use processor::{current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task};
pub use resource::*;
pub use signal::*;
pub use task::FD_LIMIT;

use crate::fs::{OpenFlags, open};

/// 将当前任务置为就绪态，放回到进程管理器中的就绪队列中，重新选择一个进程运行
pub fn suspend_current_and_run_next() -> isize{
    // There must be an application running.
    // 取出当前正在执行的任务
    let task_cp = current_task().unwrap();
    let mut task_inner = task_cp.inner_exclusive_access();
    if task_inner.signals.contains(Signals::SIGKILL){
        let exit_code = task_inner.exit_code;
        drop(task_inner);
        drop(task_cp);
        exit_current_and_run_next(exit_code);
        return 0;
    }
    let task = take_current_task().unwrap();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // 修改其进程控制块内的状态为就绪状态
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // 将进程加入进程管理器中的就绪队列
    add_task(task);
    // 开启一轮新的调度
    schedule(task_cx_ptr);
    0
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // println!("[KERNEL] pid:{} exited", current_task().unwrap().pid.0);

    // 获取访问权限，修改进程状态
    let task = take_current_task().unwrap();
    remove_from_pid2task(task.getpid());
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie; // 后续才能被父进程在 waitpid 系统调用的时候回收
                                            // 记录退出码，后续父进程在 waitpid 的时候可以收集
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    if task.getpid() == 0 {
        panic!("initproc return!");
    }

    {
        // 将这个进程的子进程转移到 initproc 进程的子进程中
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone()); // 引用计数 -1
        }
    }

    inner.children.clear(); // 引用计数 +1
                            // 对于当前进程占用的资源进行早期回收
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(task);
    // 使用全0的上下文填充换出上下文，开启新一轮进程调度
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// ### 初始进程的进程控制块
    /// - 引用计数类型，数据存放在内核堆中
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        extern "C" {
            fn _num_app();
        }
        let num_app_ptr = _num_app as usize as *mut usize;
        let num_app = unsafe { num_app_ptr.read_volatile() };
        let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
        let inode = open("/", "initproc", OpenFlags::O_CREATE).expect("initproc create failed!");
        let mut data = Vec::new();
        let initproc_data = unsafe { core::slice::from_raw_parts(app_start[0] as *const u8, app_start[1] - app_start[0]) };
        data.resize(app_start[1] - app_start[0], 0);
        data.clone_from_slice(initproc_data);
        inode.write_all(&data);
        drop(data);
        TaskControlBlock::new(inode)
    });
}

/// 将初始进程 `initproc` 加入任务管理器
pub fn add_initproc() {
    add_task(INITPROC.clone());
}
