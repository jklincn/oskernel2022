/// # 进程状态标志
/// `os/src/task/signal.rs`
/// ```
/// pub struct SignalFlags
/// ```
//

use bitflags::*;

bitflags! {
    /// 进程状态标志
    pub struct SignalFlags: u32 {   /// - Killed
        const SIGINT    = 1 << 2;   /// - Illegal Instruction
        const SIGILL    = 1 << 4;   /// - Aborted
        const SIGABRT   = 1 << 6;   /// - Erroneous Arithmetic Operation
        const SIGFPE    = 1 << 8;   /// - Segmentation Fault
        const SIGSEGV   = 1 << 11;
    }
}

impl SignalFlags {
    pub fn check_error(&self) -> Option<(i32, &'static str)> {
        if self.contains(Self::SIGINT) {
            Some((-2, "Killed, SIGINT=2"))
        } else if self.contains(Self::SIGILL) {
            Some((-4, "Illegal Instruction, SIGILL=4"))
        } else if self.contains(Self::SIGABRT) {
            Some((-6, "Aborted, SIGABRT=6"))
        } else if self.contains(Self::SIGFPE) {
            Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "Segmentation Fault, SIGSEGV=11"))
        } else {
            None
        }
    }
}
