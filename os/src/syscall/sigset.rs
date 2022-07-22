use crate::mm::{translated_byte_buffer, UserBuffer};
use crate::task::{current_task, current_user_token, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};

pub fn sys_rt_sigprocmask(how: i32, set: *const usize, oldset: *const usize, sigsetsize: usize) -> isize {
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

pub fn sys_rt_sigreturn(setptr: *mut usize) -> isize {
    0
}

pub fn sys_rt_sigaction() -> isize {
    0
}

pub fn sys_rt_sigtimedwait() -> isize {
    0
}

pub fn sys_futex() -> isize {
    0
}
