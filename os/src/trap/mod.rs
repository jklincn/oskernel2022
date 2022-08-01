/// # 提供 `Trap` 管理
/// `os/src/trap/mod.rs`
/// ## 实现功能
/// ```
/// pub fn init()
/// pub fn enable_timer_interrupt()
/// pub fn trap_handler() -> ! 
/// pub fn trap_return() -> !
/// pub fn trap_from_kernel() -> !
/// ```
//

mod context;

use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::syscall::syscall;
use crate::task::{
    check_signals_of_current, current_add_signal, current_trap_cx, current_user_token,
    exit_current_and_run_next, suspend_current_and_run_next, SignalFlags,current_task
};
use crate::timer::set_next_trigger;
use crate::mm::VirtAddr;
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

/// ### 设置内核态下的 Trap 入口
/// 一旦进入内核后再次触发到 S态 Trap，则硬件在设置一些 CSR 寄存器之后，会跳过对通用寄存器的保存过程，
/// 直接跳转到 trap_from_kernel 函数，在这里直接 panic 退出
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

/// ### 设置用户态下的 Trap 入口
/// 我们把 stvec 设置为内核和应用地址空间共享的跳板页面的起始地址 TRAMPOLINE 
/// 而不是编译器在链接时看到的 __alltraps 的地址。这是因为启用分页模式之后，
/// 内核只能通过跳板页面上的虚拟地址来实际取得 __alltraps 和 __restore 的汇编代码
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
    let scause = scause::read();    // 用于描述 Trap 的原因
    let stval = stval::read();       // 给出 Trap 附加信息
    match scause.cause() {
        // 触发 Trap 的原因是来自 U 特权级的 Environment Call，也就是系统调用
        Trap::Exception(Exception::UserEnvCall) => {
            // 由于应用的 Trap 上下文不在内核地址空间，因此我们调用 current_trap_cx 来获取当前应用的 Trap 上下文的可变引用
            let mut cx = current_trap_cx();
            cx.sepc += 4;   // 我们希望trap返回后应用程序从下一条指令开始执行
            // 从 Trap 上下文取出作为 syscall ID 的 a7 和系统调用的三个参数 a0~a2 传给 syscall 函数并获取返回值放到 a0
            // println!("syscall_id: {}",cx.x[17]);
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]]);
            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }

        // 处理应用程序出现访存错误，判断能否CoW/lazy
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            let is_load: bool;
            if scause.cause() == Trap::Exception(Exception::LoadFault) || scause.cause() == Trap::Exception(Exception::LoadPageFault) {
                is_load = true;
            } else {
                is_load = false;
            }
            println!("stval:0x{:?}",stval::read());
            let va: VirtAddr = (stval as usize).into();
            if va > TRAMPOLINE.into() {
                println!("[kernel] VirtAddr out of range!");
                current_add_signal(SignalFlags::SIGSEGV);
            }

            let lazy = current_task().unwrap().check_lazy(va, is_load);
            if lazy != 0 { 
                current_add_signal(SignalFlags::SIGSEGV); 
            }

            // current_trap_cx().debug_show();
            // current_task().unwrap().inner_exclusive_access().task_cx.debug_show();
            // current_task().unwrap().inner_exclusive_access().memory_set.debug_show_data(TRAP_CONTEXT.into());
        }

        Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
            let task = current_task().unwrap();
            println!(
                "[kernel] {:?} in application {}, bad addr = {:#x}, bad instruction = {:#x}.",
                scause.cause(),
                task.pid.0,
                stval,
                current_trap_cx().sepc,
            );
            drop(task);

            current_trap_cx().debug_show();
            // current_task().unwrap().inner_exclusive_access().task_cx.debug_show();
            
            //current_task().unwrap().inner_exclusive_access().memory_set.debug_show_data(TRAP_CONTEXT.into());

            current_add_signal(SignalFlags::SIGSEGV);
        }

        // 处理应用程序出现非法指令错误
        Trap::Exception(Exception::IllegalInstruction) => {
            // println!("[kernel] IllegalInstruction in application, kernel killed it.");
            // // illegal instruction exit code
            // exit_current_and_run_next(-3);
            println!("stval:{}",stval);
            let sepc =riscv::register::sepc::read();
            println!("sepc:0x{:x}",sepc);
            // current_task().unwrap().inner_exclusive_access().memory_set.debug_show_data(sepc.into());
            current_add_signal(SignalFlags::SIGILL);
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
    
    // check signals
    if let Some((errno, msg)) = check_signals_of_current() {
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }
    trap_return();
}

/// 通过在Rust语言中加入宏命令调用 `__restore` 汇编函数
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
            "fence.i",              // 指令清空指令缓存 i-cache
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,   // Trap 上下文在应用地址空间中的位置
            in("a1") user_satp,     // 即将回到的应用的地址空间的 token
            options(noreturn)
        );
    }
}

/// 在内核触发Trap后会转到这里引发Panic
#[no_mangle]
pub fn trap_from_kernel() -> ! {
    use riscv::register::sepc;
    println!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}

pub use context::TrapContext;
