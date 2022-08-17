use alloc::vec::Vec;

use crate::mm::{translated_byte_buffer, translated_refmut, UserBuffer};
use crate::task::{current_task, current_trap_cx, current_user_token, pid2task, Signals, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK, SigAction, SaFlags};

pub fn sys_kill(pid: usize, signal: u32) -> isize {
    // println!("[KERNEL] enter sys_kill: pid:{} send to pid:{}, signal:0x{:x}",current_task().unwrap().pid.0, pid, signal);
    if signal == 0 {
        return 0;
    }
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = Signals::from_bits(1 << signal) {
            task.inner_exclusive_access().signals |= flag;
            0
        } else {
            panic!("[DEBUG] sys_kill: unsupported signal");
        }
    } else {
        // panic!("[DEBUG] sys_kill: pid does not exist");
        0 // busybox hwclock will kill pid 10
    }
}

pub fn sys_rt_sigprocmask(how: i32, set: *const usize, oldset: *const usize, _sigsetsize: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    // println!("enter sys_rt_sigprocmask!");
    // println!("how:{},set:{:?},oldset:{:?}",how,set,oldset);
    if oldset as usize != 0 {
        let inner = task.inner_exclusive_access();
        let buf_vec = translated_byte_buffer(token, oldset as *const u8, 128);
        let mut userbuf = UserBuffer::new(buf_vec);
        userbuf.write(inner.sigset.bits.as_slice());
    }
    if set as usize != 0 {
        let mut buf_vec = translated_byte_buffer(token, set as *const u8, 128);
        let buf = buf_vec.pop().unwrap();
        let mut inner = task.inner_exclusive_access();
        for (i, v) in inner.sigset.bits.iter_mut().enumerate() {
            match how {
                SIG_BLOCK => *v = *v | buf[i],
                SIG_UNBLOCK => *v = *v & buf[i],
                SIG_SETMASK => *v = buf[i],
                _ => panic!("sys_rt_sigprocmask: unsupported how"),
            }
        }
    }
    0
}

// DEBUG
pub fn sys_rt_sigaction(signal: isize, act: *mut usize, oldact: *mut usize) -> isize {
    // println!(
    //     "[DEBUG] enter sys_rt_sigaction: signum:{}, act:{}, oldact:{}",
    //     signal, act as usize, oldact as usize
    // );
    // let task = current_task().unwrap();
    // let mut inner = task.inner_exclusive_access();
    // let token = current_user_token();
    // let signal = Signals::from_bits(1 << signal).expect("[DEBUG] sys_rt_sigaction: unsupported signal");

    // if oldact as usize != 0 {
    //     if let Some(sigaction) = inner.sigaction.remove(&signal) {
    //         let old_sigaction = sigaction;
    //         *translated_refmut(token, oldact) = old_sigaction.sa_handler;
    //         *translated_refmut(token, unsafe { oldact.add(1) }) = old_sigaction.sa_flags.bits();
    //         *translated_refmut(token, unsafe { oldact.add(2) }) = {
    //             if old_sigaction.sa_mask.is_empty() {
    //                 0
    //             } else {
    //                 old_sigaction.sa_mask[0].bits() as usize
    //             }
    //         };
    //     } else {
    //         *translated_refmut(token, oldact) = 0;
    //         *translated_refmut(token, unsafe { oldact.add(1) }) = 0;
    //         *translated_refmut(token, unsafe { oldact.add(2) }) = 0;
    //     }
    // }

    // if act as usize != 0 {
    //     let handler = *translated_refmut(token, act);
    //     let _flags = *translated_refmut(token, unsafe { act.add(1) });
    //     let mask = *translated_refmut(token, unsafe { act.add(2) });
    //     let mut sigaction_new = SigAction {
    //         sa_handler: handler,
    //         sa_mask: Vec::new(),
    //         sa_flags: SaFlags::SA_RESTART,
    //     };
    //     if mask != 0 {
    //         sigaction_new.sa_mask.push(Signals::from_bits(mask as u32).unwrap());
    //     }
    //     inner.sigaction.insert(signal, sigaction_new);
    // }
    0
}

pub fn sys_rt_sigtimedwait() -> isize {
    0
}

pub fn sys_rt_sigreturn(_setptr: *mut usize) -> isize {
    // let trap_cx = current_trap_cx();
    // let current_task = current_task().unwrap();
    // let inner = current_task.inner_exclusive_access();
    // // restore trap_cx
    // *trap_cx = inner.trapcx_backup.clone();
    // return trap_cx.x[10] as isize;
    0
}