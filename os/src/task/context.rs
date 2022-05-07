/// # 任务上下文模块
/// `os/src/task/context.rs`
/// ```
/// pub struct TaskContext
/// TaskContext::zero_init() -> Self
/// TaskContext::goto_trap_return(kstack_ptr: usize) -> Self
/// ```
//

use crate::trap::trap_return;

/// ### 任务上下文
/// - `ra`:返回后PC的位置
/// - `sp`:栈顶指针
/// - `s`:`s[0]~s[11]`
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    /// 初始化任务上下文为0
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// 当每个应用第一次获得 CPU 使用权即将进入用户态执行的时候，它的内核栈顶放置着我们在
    /// 内核加载应用的时候构造的一个任务上下文,在 `__switch` 切换到该应用的任务上下文的时候，
    /// 内核将会跳转到 `trap_return` 并返回用户态开始该应用的启动执行
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
