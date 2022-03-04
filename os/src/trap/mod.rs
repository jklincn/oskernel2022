/// # 提供 `Trap` 管理
/// `os/src/trap/mod.rs`
/// ## 实现功能
/// ```
/// pub fn init()
/// pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext
/// ```
//

mod context;

use crate::batch::run_next_app;
use crate::syscall::syscall;
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};

// 我们在 os/src/trap/trap.S 中实现 Trap 上下文保存/恢复的汇编代码，
// 分别用外部符号 __alltraps 和 __restore 标记为函数，
// 并通过 global_asm! 宏将 trap.S 这段汇编代码插入进来。
global_asm!(include_str!("trap.S"));
// Trap 处理的总体流程如下：首先通过 __alltraps 将 Trap 上下文保存在内核栈上，
// 然后跳转到使用 Rust 编写的 trap_handler 函数完成 Trap 分发及处理。
// 当 trap_handler 返回之后，使用 __restore 从保存在内核栈上的 Trap 上下文恢复寄存器。
// 最后通过一条 sret 指令回到应用程序执行。

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        // stvec:控制 Trap 处理代码的入口地址
        // 将 stvec 设置为 Direct 模式, 指向它的地址
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// ### `trap` 处理函数
#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
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
            println!("[kernel] PageFault in application, kernel killed it.");
            run_next_app();
        }
        // 处理应用程序出现非法指令错误
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            run_next_app();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::TrapContext;
