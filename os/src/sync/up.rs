/// ### 允许在单核处理器上将引用做全局变量使用
/// `os/src/sync/up.rs`
//

use core::cell::{RefCell, RefMut};

///在里面包一个静态数据结构，这样我们就可以在任何没有 `unsafe` 标签的情况下访问它
/// 
/// 我们应该只在单处理器中使用它。
///
/// 要获取内部数据的可变引用，请按规则调用 `exclusive_access`.
/// 
/// ### 规则：访问之前调用 exclusive_access ，访问之后销毁借用标记再进行下一次访问
pub struct UPSafeCell<T> {
    /// 内部数据
    inner: RefCell<T>,
}

// 将 UPSafeCell 标记为 Sync 使得它可以作为一个全局变量, 这是 unsafe 行为
unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// 当使用者违背了规则，比如访问之后忘记销毁就开启下一次访问时，程序会 panic 并退出。
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    /// 获取`UPSafeCell`中数据的独占访问权，
    /// 如果数据已被借用，则会引发 `panic`
    /// 
    /// 在操作完成之后我们需要销毁这个标记，此后才能开始对该数据的下一次访问
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
