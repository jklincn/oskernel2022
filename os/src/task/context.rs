/// # 任务上下文模块
/// `os/src/task/context.rs`
/// ```
/// pub struct TaskContext
/// TaskContext::zero_init() -> Self
/// TaskContext::goto_restore(kstack_ptr: usize) -> Self
/// ```
//

#[derive(Copy, Clone)]
#[repr(C)]

/// ### 任务上下文
/// - `ra`:
/// - `sp`:
/// - `s`:`s[0]~s[11]`
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
    /// 构造任务保存在任务控制块中的任务上下文
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            // 设置任务上下文中的内核栈指针将任务上下文的 ra 寄存器设置为 __restore 的入口地址
            // 这样，在 __switch 从它上面恢复并返回之后就会直接跳转到 __restore
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
