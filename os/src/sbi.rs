// os/src/sbi/rs
use core::arch::asm;

//定义 RustSBI 支持的服务类型常量
#[allow(unused)]
const SBI_SET_TIMER:                usize = 0;
const SBI_CONSOLE_PUTCHAR:          usize = 1;
const SBI_CONSOLE_GETCHAR:          usize = 2;
const SBI_CLEAR_IPI:                usize = 3;
const SBI_SEND_IPI:                 usize = 4;
const SBI_REMOTE_FENCE_I:           usize = 5;
const SBI_REMOTE_SFENCE_VMA:        usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID:   usize = 7;
const SBI_SHUTDOWN:                 usize = 8;

#[inline(always)]
// which 表示请求 RustSBI 的服务的类型
// arg0 ~ arg2 表示传递给 RustSBI 的 3 个参数
// RustSBI 在将请求处理完毕后，会给内核一个返回值，这个返回值也会被 sbi_call 函数返回
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

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN,0 ,0 ,0);
    panic!("It shuld shudtdown!");
}