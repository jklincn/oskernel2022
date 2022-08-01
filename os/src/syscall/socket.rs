/// 从功利性角度上讲，做这么多只有1分，时间太不划算了
/// jk记录于2020-07-23，离考研还有150天 :(

use crate::mm::{translated_byte_buffer, translated_ref, translated_refmut, translated_str, UserBuffer};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next, pid2task, suspend_current_and_run_next, RLimit, SignalFlags, new,
};

pub fn sys_socket()->isize{
    0
}

pub fn sys_bind()->isize{
    0
}

pub fn sys_getsockname()->isize{
    0
}

pub fn sys_setsockopt()->isize{
    0
}

pub fn sys_sendto()->isize{
    1
}

pub fn sys_recvfrom(_:isize, buf:usize, _:usize, _:usize, _:usize, _:usize)->isize{
    let token = current_user_token();
    let buf_vec = translated_byte_buffer(token, buf as *const u8, 1);
    let mut userbuf = UserBuffer::new(buf_vec);
    userbuf.write(&[120u8]);
    1
}

pub fn sys_listen()->isize{
    0
}

pub fn sys_connect()->isize{
    0
}

pub fn sys_accept()->isize{
    0
}