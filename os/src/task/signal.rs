/// # 进程状态标志
/// `os/src/task/signal.rs`
/// ```
/// pub struct SignalFlags
/// ```
//
use bitflags::*;

use super::current_task;

bitflags! {
    /// 进程状态标志
    pub struct SignalFlags: u32 {   /// - Killed
        const SIGINT    = 1 << 2;   /// - Illegal Instruction
        const SIGILL    = 1 << 4;   /// - Aborted
        const SIGABRT   = 1 << 6;   /// - Erroneous Arithmetic Operation
        const SIGFPE    = 1 << 8;   /// - Segmentation Fault
        const SIGKILL   = 1 << 9;
        const SIGUSR1   = 1 << 10;
        const SIGSEGV   = 1 << 11;
    }
}

pub fn check_signals_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    match task_inner.signals{
        SignalFlags::SIGINT => Some((-2, "Killed, SIGINT=2")),
        SignalFlags::SIGILL => Some((-4, "Illegal Instruction, SIGILL=4")),
        SignalFlags::SIGABRT=> Some((-6, "Aborted, SIGABRT=6")),
        SignalFlags::SIGFPE => Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8")),
        SignalFlags::SIGKILL =>Some((-9, "Kill, SIGKILL=9")),
        SignalFlags::SIGSEGV=> Some((-11, "Segmentation Fault, SIGSEGV=11")),
        _ => None
    }
}

pub fn current_add_signal(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
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
