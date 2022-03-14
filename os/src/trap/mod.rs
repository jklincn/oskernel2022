/// # 提供 `Trap` 管理
/// `os/src/trap/mod.rs`
/// ## 实现功能
/// ```
/// pub fn init()
/// pub fn trap_handler() -> ! 
/// ```
//

mod context;

use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::syscall::syscall;
use crate::task::{
    current_trap_cx, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
};
use crate::timer::set_next_trigger;
use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

// 我们在 os/src/trap/trap.S 中实现 Trap 上下文保存/恢复的汇编代码，
// 分别用外部符号 __alltraps 和 __restore 标记为函数，
// 并通过 global_asm! 宏将 trap.S 这段汇编代码插入进来。
global_asm!(include_str!("trap.S"));
// Trap 处理的总体流程如下：首先通过 __alltraps 将 Trap 上下文（不是那个结构体）保存在内核栈上，
// 然后跳转到使用 Rust 编写的 trap_handler 函数完成 Trap 分发及处理。
// 当 trap_handler 返回之后，使用 __restore 从保存在内核栈上的 Trap 上下文恢复寄存器。
// 最后通过一条 sret 指令回到应用程序执行。

pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

/// 启用 S 特权级时钟中断
pub fn enable_timer_interrupt() {
    unsafe {
        // 启用 S 特权级时钟中断
        sie::set_stimer();
    }
}

/// ### `trap` 处理函数
#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let cx = current_trap_cx();
    let scause = scause::read();    // 用于描述 Trap 的原因
    let stval = stval::read();       // 给出 Trap 附加信息
    match scause.cause() {
        // 触发 Trap 的原因是来自 U 特权级的 Environment Call，也就是系统调用
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;   // 我们希望trap返回后应用程序从下一条指令开始执行
            // 从 Trap 上下文取出作为 syscall ID 的 a7 和系统调用的三个参数 a0~a2 传给 syscall 函数并获取返回值 放到 a0
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        // 处理应用程序出现访存错误
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        // 处理应用程序出现非法指令错误
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }
        // 时间片到中断
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    // __restore 在虚拟地址空间的地址
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,   // Trap 上下文在应用地址空间中的位置
            in("a1") user_satp,     // 即将回到的应用的地址空间的 token
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

pub use context::TrapContext;
