/// # SBI相关模块
/// `os/src/sbi.rs`
/// ## 实现功能
/// ```
/// pub fn console_putchar(c: usize)
/// pub fn console_getchar() -> usize
/// pub fn shutdown()
/// ```
//

use core::arch::asm;

//定义 RustSBI 支持的服务类型常量
const SBI_SET_TIMER:                usize = 0;
const SBI_CONSOLE_PUTCHAR:          usize = 1;
const SBI_CONSOLE_GETCHAR:          usize = 2;
#[allow(unused)]
const SBI_CLEAR_IPI:                usize = 3;
#[allow(unused)]
const SBI_SEND_IPI:                 usize = 4;
#[allow(unused)]
const SBI_REMOTE_FENCE_I:           usize = 5;
#[allow(unused)]
const SBI_REMOTE_SFENCE_VMA:        usize = 6;
#[allow(unused)]
const SBI_REMOTE_SFENCE_VMA_ASID:   usize = 7;
const SBI_SHUTDOWN:                 usize = 8;

/// ### SBI调用
/// - `which` 表示请求 RustSBI 的服务的类型
/// - `arg0` ~ `arg2` 表示传递给 RustSBI 的 3 个参数
/// - `RustSBI` 在将请求处理完毕后，会给内核一个返回值，这个返回值也会被 sbi_call 函数返回
#[inline(always)]
fn sbi_call(whitch: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe{
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") whitch,
        );
    }
    ret
}

/// ### 设置 `mtimecmp` 的值
/// - 由 SEE 提供的标准 SBI 接口函数
/// - `mtimecmp`:一旦计数器 `mtime` 的值超过了 `mtimecmp`，就会触发一次时钟中断
/// - `mtime`: 统计处理器自上电以来经过了多少个内置时钟的时钟周期,64bit
pub fn set_timer(timer: usize) {
    sbi_call(SBI_SET_TIMER, timer, 0, 0);
}

/// ### 向终端输出一个字符
/// - 采用`sbi_call()`实现
/// - 参数
///     - `c`: 待输出的字符
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

/// ### 从终端读取一个字符
/// - 采用`sbi_call()`实现
/// - 无参数
/// - 返回一个字符或者 0 ，表示没读到
pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

/// ### 调用SBI_Call关机
/// - 采用sbi_call实现
/// - 若关机失败则引发异常
pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN,0 ,0 ,0);
    panic!("It should shutdown!");
}