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
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
