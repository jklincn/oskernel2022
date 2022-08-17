use alloc::vec::Vec;
/// # 进程信号机制
/// `os/src/task/signal.rs`
/// ```
/// pub struct SignalFlags
/// ```
//
use bitflags::*;

use super::current_task;

bitflags! {
    /// 进程信号机制
    /// 和linux不同的是，这样设计不会出现 1 << 0 
    pub struct Signals: u32 {
        const SIGINT    = 1 << 2;   // 中断
        const SIGQUIT   = 1 << 3;   // （未处理）退出
        const SIGILL    = 1 << 4;   // 非法指令
        const SIGABRT   = 1 << 6;   // abort发出的信号
        const SIGFPE    = 1 << 8;   // 浮点异常
        const SIGKILL   = 1 << 9;   // Kill信号，不能被忽略、处理和阻塞
        const SIGUSR1   = 1 << 10;  // 用户信号1
        const SIGSEGV   = 1 << 11;  // 无效内存访问
        const SIGTERM   = 1 << 15;  // （未处理）终止信号
        const SIGCHLD   = 1 << 17;  // （未处理）子进程相关
        const SIGSTOP   = 1 << 19;  // （未处理）进程停止
    }
}

pub fn check_signals_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    // println!("pid:{},signal:{:?}",task.getpid(),task_inner.signals);
    match task_inner.signals{
        Signals::SIGINT => Some((-2, "Killed, SIGINT=2")),
        Signals::SIGILL => Some((-4, "Illegal Instruction, SIGILL=4")),
        Signals::SIGABRT=> Some((-6, "Aborted, SIGABRT=6")),
        Signals::SIGFPE => Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8")),
        Signals::SIGKILL=> Some((-9, "Kill, SIGKILL=9")),
        Signals::SIGSEGV=> Some((-11, "Segmentation Fault, SIGSEGV=11")),
        _ => None
    }
}

// pub fn scan_signal_handler_of_current() -> Option<(Signals,usize)>{
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();

//     let signal_handler = inner.sigaction.clone();
//     while !inner.siginfo.signal_pending.is_empty() {
//         let signum = inner.siginfo.signal_pending.pop().unwrap();
//         if let Some(sigaction) = signal_handler.get(&signum){
//             if sigaction.sa_handler == 0{
//                 continue;
//             }
//             {// avoid borrow mut trap_cx, because we need to modify trapcx_backup
//                 let trap_cx = inner.get_trap_cx().clone();
//                 inner.trapcx_backup = trap_cx;          // backup
//             }
//             {
//                 let trap_cx = inner.get_trap_cx();
//                 trap_cx.set_sp(USER_SIGNAL_STACK);      // sp-> signal_stack
//                 trap_cx.x[10] = log2(signum.bits());    // a0=signum
//                 trap_cx.x[1] = SIGNAL_TRAMPOLINE;       // ra-> signal_trampoline
//                 trap_cx.sepc = sigaction.sa_handler;    // sepc-> sa_handler
//             }
//             inner.siginfo.is_signal_execute = true;
//             return Some((signum, sigaction.sa_handler));
//         }   
//         else{// check SIGTERM independently
//             if signum == Signals::SIGTERM || signum == Signals::SIGKILL{
//                 return Some((signum, SIG_DFL));
//             }
//         }
//     }
//     None
// }

pub fn current_add_signal(signal: Signals) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
}

#[derive(Clone)]
pub struct SigAction {
    pub sa_handler:usize,
    // pub sa_sigaction:usize,
    pub sa_mask:Vec<Signals>,
    pub sa_flags:SaFlags,
}

bitflags!{
    /* Bits in `sa_flags'.  */
    pub struct SaFlags: usize{
        const SA_NOCLDSTOP = 1		   ;     /* Don't send SIGCHLD when children stop.  */
        const SA_NOCLDWAIT = 2		   ;     /* Don't create zombie on child death.  */
        const SA_SIGINFO   = 4		   ;     /* Invoke signal-catching function with three arguments instead of one.  */
        const SA_ONSTACK   = 0x08000000;    /* Use signal stack by using `sa_restorer'. */
        const SA_RESTART   = 0x10000000;    /* Restart syscall on signal return.  */
        const SA_NODEFER   = 0x40000000;    /* Don't automatically block the signal when its handler is being executed.  */
        const SA_RESETHAND = 0x80000000;    /* Reset to SIG_DFL on entry to handler.  */
        const SA_INTERRUPT = 0x20000000;    /* Historical no-op.  */
    }
}

// pub const SIGHUP: u32 = 1;
// pub const SIGINT: u32 = 2;
// pub const SIGQUIT: u32 = 3;
// pub const SIGILL: u32 = 4;
// pub const SIGTRAP: u32 = 5;
// pub const SIGABRT: u32 = 6;
// pub const SIGIOT: u32 = SIGABRT;
// pub const SIGBUS: u32 = 7;
// pub const SIGFPE: u32 = 8;
// pub const SIGKILL: u32 = 9;
// pub const SIGUSR1: u32 = 10;
// pub const SIGSEGV: u32 = 11;
// pub const SIGUSR2: u32 = 12;
// pub const SIGPIPE: u32 = 13;
// pub const SIGALRM: u32 = 14;
// pub const SIGTERM: u32 = 15;
// pub const SIGSTKFLT: u32 = 16;
// pub const SIGCHLD: u32 = 17;
// pub const SIGCONT: u32 = 18;
// pub const SIGSTOP: u32 = 19;
// pub const SIGTSTP: u32 = 20;
// pub const SIGTTIN: u32 = 21;
// pub const SIGTTOU: u32 = 22;
// pub const SIGURG: u32 = 23;
// pub const SIGXCPU: u32 = 24;
// pub const SIGXFSZ: u32 = 25;
// pub const SIGVTALRM: u32 = 26;
// pub const SIGPROF: u32 = 27;
// pub const SIGWINCH: u32 = 28;
// pub const SIGIO: u32 = 29;
// pub const SIGPOLL: u32 = SIGIO;
// pub const SIGPWR: u32 = 30;
// pub const SIGSYS: u32 = 31;
// pub const SIGUNUSED: u32 = SIGSYS;

pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;

// pub const SIGSET_LEN: usize = 1024 / (8 * (core::mem::size_of::<usize>()));

pub struct SigSet {
    // pub bits: [usize; SIGSET_LEN],
    pub bits: [u8; 128],
}

impl SigSet {
    pub fn new() -> Self {
        Self { bits: [0; 128] }
    }
}
