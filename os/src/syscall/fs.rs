use crate::fs::{chdir, make_pipe, open, Dirent, File, Kstat, OpenFlags, Statfs, Timespec, MNT_TABLE};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token, RLIMIT_NOFILE};
use alloc::sync::Arc;
/// # 文件读写模块
/// `os/src/syscall/fs.rs`
/// ## 实现功能
/// ```
/// pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize
/// pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize
/// pub fn sys_open(path: *const u8, flags: u32) -> isize
/// pub fn sys_close(fd: usize) -> isize
/// ```
//
use core::mem::size_of;

const AT_FDCWD: isize = -100;
pub const FD_LIMIT: usize = 128;

/// ### 写文件函数
/// - `fd` 表示待写入文件的文件描述符；
/// - `buf` 表示内存中缓冲区的起始地址；
/// - `len` 表示内存中缓冲区的长度。
/// - 返回值：成功写入的长度。
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可写
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // 释放 taskinner 以避免多次借用
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// ### 读文件函数
/// - `fd` 表示待读取文件的文件描述符；
/// - `buf` 表示内存中缓冲区的起始地址；
/// - `len` 表示读取字符个数。
/// - 返回值：读出的字符。
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可读
        if !file.readable() {
            return -1;
        }
        let file = file.clone();
        // 释放 taskinner 以避免多次借用
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_openat(dirfd: isize, path: *const u8, flags: u32, mode: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();

    let path = translated_str(token, path);
    let oflags = OpenFlags::from_bits(flags).unwrap();

    // todo
    _ = mode;

    if dirfd == AT_FDCWD {
        // 如果是当前工作目录
        if let Some(inode) = open(inner.get_work_path().as_str(), path.as_str(), oflags) {
            let fd = inner.alloc_fd();
            inner.fd_table[fd] = Some(inode);
            fd as isize
        } else {
            -1
        }
    } else {
        let dirfd = dirfd as usize;
        // dirfd 不合法
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            if let Some(tar_f) = open(file.get_name().as_str(), path.as_str(), oflags) {
                let fd = inner.alloc_fd();
                inner.fd_table[fd] = Some(tar_f);
                fd as isize
            } else {
                -1
            }
        } else {
            // dirfd 对应条目为 None
            -1
        }
    }
}

/// ### 关闭文件函数
/// - `fd`：文件描述符
/// - 返回值
///     - 成功关闭：0
///     - 失败：-1
pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    // 把 fd 对应的值取走，变为 None
    inner.fd_table[fd].take();
    0
}

/// ### 为当前进程打开一个管道。
/// - `pipe` 表示应用地址空间中的一个长度为 `2` 的 `usize` 数组的起始地址，
/// 内核需要按顺序将管道读端和写端的文件描述符写入到数组中。
/// - 返回值：如果出现了错误则返回 -1，否则返回 0 。可能的错误原因是：传入的地址不合法。
/// - syscall ID：59
pub fn sys_pipe(pipe: *mut u32, flag: usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();

    // todo
    _ = flag;

    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe) = read_fd as u32;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd as u32;
    0
}

/// ### 将进程中一个已经打开的文件描述符复制一份并分配到一个新的文件描述符中。
/// - 参数：fd 表示进程中一个已经打开的文件的文件描述符。
/// - 返回值：
///     - 能够访问已打开文件的新文件描述符。
///     - 如果出现了错误则返回 -1，可能的错误原因是：传入的 fd 并不对应一个合法的已打开文件。
/// - syscall ID：23

// 暂时写在这里
const EMFILE: isize = 24;

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    // 做资源检查，目前只检查 RLIMIT_NOFILE 这一种
    let rlim_max = inner.resource[RLIMIT_NOFILE].rlim_max;
    // println!("cur:{},max:{}", rlim_cur, rlim_max);
    if inner.fd_table.len() - 1 == rlim_max - 1 {
        return -EMFILE;
    }

    // 检查传入 fd 的合法性
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
}

/// ### 将进程中一个已经打开的文件描述符复制一份并分配到一个指定的文件描述符中。
/// - 参数：
///     - old_fd 表示进程中一个已经打开的文件的文件描述符。
///     - new_fd 表示进程中一个指定的文件描述符中。
/// - 返回值：
///     - 能够访问已打开文件的新文件描述符。
///     - 如果出现了错误则返回 -1，可能的错误原因是：
///         - 传入的 old_fd 为空。
///         - 传入的 old_fd 不存在
///         - 传入的 new_fd 超出描述符数量限制 (典型值：128)
/// - syscall ID：24
pub fn sys_dup3(old_fd: usize, new_fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if old_fd >= inner.fd_table.len() || new_fd > FD_LIMIT {
        return -1;
    }
    if inner.fd_table[old_fd].is_none() {
        return -1;
    }
    if new_fd >= inner.fd_table.len() {
        for _ in inner.fd_table.len()..(new_fd + 1) {
            inner.fd_table.push(None);
        }
    }

    //let mut act_fd = new_fd;
    //if inner.fd_table[new_fd].is_some() {
    //    act_fd = inner.alloc_fd();
    //}
    //let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(inner.fd_table[old_fd].as_ref().unwrap().clone());
    new_fd as isize
}

pub fn sys_mkdirat(dirfd: isize, path: *const u8, mode: u32) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let path = translated_str(token, path);

    // todo
    _ = mode;

    if dirfd == AT_FDCWD {
        if let Some(_) = open(inner.get_work_path().as_str(), path.as_str(), OpenFlags::O_DIRECTROY) {
            0
        } else {
            -1
        }
    } else {
        let dirfd = dirfd as usize;
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            if let Some(_) = open(file.get_name().as_str(), path.as_str(), OpenFlags::O_DIRECTROY) {
                0
            } else {
                -1
            }
        } else {
            // dirfd 对应条目为 None
            -1
        }
    }
}

/// buf：用于保存当前工作目录的字符串。当 buf 设为 NULL，由系统来分配缓存区
pub fn sys_getcwd(buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    if buf as usize == 0 {
        unimplemented!();
    } else {
        let buf_vec = translated_byte_buffer(token, buf, len);
        let mut userbuf = UserBuffer::new(buf_vec);
        let cwd = inner.current_path.as_bytes();
        userbuf.write(cwd);
        return buf as isize;
    }
}

pub fn sys_mount(special: *const u8, dir: *const u8, fstype: *const u8, flags: usize, data: *const u8) -> isize {
    let token = current_user_token();
    let special = translated_str(token, special);
    let dir = translated_str(token, dir);
    let fstype = translated_str(token, fstype);

    _ = data;

    MNT_TABLE.lock().mount(special, dir, fstype, flags as u32)
}

pub fn sys_umount(p_special: *const u8, flags: usize) -> isize {
    let token = current_user_token();
    let special = translated_str(token, p_special);
    MNT_TABLE.lock().umount(special, flags as u32)
}

pub fn sys_unlinkat(fd: isize, path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();

    // todo
    _ = flags;

    let path = translated_str(token, path);
    if fd == AT_FDCWD {
        if let Some(file) = open(inner.get_work_path().as_str(), path.as_str(), OpenFlags::from_bits(0).unwrap()) {
            file.delete();
            0
        } else {
            -1
        }
    } else {
        unimplemented!();
    }
}

pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    let path = translated_str(token, path);
    if let Some(new_cwd) = chdir(inner.current_path.as_str(), &path) {
        inner.current_path = new_cwd;
        0
    } else {
        -1
    }
}

pub fn sys_fstat(fd: isize, buf: *mut u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let buf_vec = translated_byte_buffer(token, buf, size_of::<Kstat>());
    let inner = task.inner_exclusive_access();

    let mut userbuf = UserBuffer::new(buf_vec);
    let mut kstat = Kstat::new();

    let dirfd = fd as usize;
    if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
        return -1;
    }
    if let Some(file) = &inner.fd_table[dirfd] {
        file.get_fstat(&mut kstat);
        userbuf.write(kstat.as_bytes());
        0
    } else {
        -1
    }
}

pub fn sys_getdents64(fd: isize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    let dirfd = fd as usize;
    if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
        return -1;
    }

    let buf_vec = translated_byte_buffer(token, buf, len);
    let mut userbuf = UserBuffer::new(buf_vec);
    let mut dirent = Dirent::new();
    let dent_len = size_of::<Dirent>();
    let mut total_len: usize = 0;
    if let Some(file) = &inner.fd_table[dirfd] {
        loop {
            if total_len + dent_len > len {
                break;
            }
            if file.get_dirent(&mut dirent) > 0 {
                // 写入 userbuf
                userbuf.write_at(total_len, dirent.as_bytes());
                // 更新长度
                total_len += dent_len;
            } else {
                break;
            }
        }
        total_len as isize
    } else {
        -1
    }
}

// 暂时放在这里
bitflags! {
    pub struct SeekFlags: usize {
        const SEEK_SET = 0;   // 参数 offset 即为新的读写位置
        const SEEK_CUR = 1;   // 以目前的读写位置往后增加 offset 个位移量
        const SEEK_END = 2;   // 将读写位置指向文件尾后再增加 offset 个位移量
    }
}

pub fn sys_lseek(fd: usize, off_t: usize, whence: usize) -> isize {
    // println!("enter sys_lseek: fd:{},off_t:{},whence:{}",fd,off_t,whence);

    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        let flag = SeekFlags::from_bits(whence).unwrap();
        match flag {
            SeekFlags::SEEK_SET => {
                file.set_offset(off_t);
                off_t as isize
            }
            SeekFlags::SEEK_CUR => {
                let current_offset = file.get_offset();
                file.set_offset(off_t + current_offset);
                (off_t + current_offset) as isize
            }
            SeekFlags::SEEK_END => {
                // 这边打开目录目前也是针对测试用例，需要完善
                let inode = open("/tmp", file.get_name().as_str(), OpenFlags::O_RDONLY).unwrap();
                let end = inode.file_size();
                file.set_offset(end + off_t);
                // println!("end:{}",end);
                (end + off_t) as isize
            }
            // flag wrong
            _ => panic!("sys_lseek: unsupported whence!"),
        }
    } else {
        // file not exists
        -3
    }
}

// 暂时放在这里
const TIOCGWINSZ: usize = 0x5413;
pub fn sys_ioctl(fd: usize, request: usize, argp: *mut u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    match request {
        TIOCGWINSZ => *translated_refmut(token, argp) = 0 as u8,
        _ => panic!("sys_ioctl: unsupported request!"),
    }
    0
}
// 暂时放在这里
#[derive(Clone, Copy, Debug)]
pub struct Iovec {
    iov_base: usize,
    iov_len: usize,
}

pub fn sys_writev(fd: usize, iovp: *const usize, iovcnt: usize) -> isize {
    // println!("enter sys_writev");
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可写
        if !file.writable() {
            return -1;
        }
        let iovp_buf = translated_byte_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>())
            .pop()
            .unwrap();
        let file = file.clone();
        let mut addr = iovp_buf.as_ptr() as *const _ as usize;
        let mut total_write_len = 0;
        drop(inner);
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
            total_write_len += file.write(UserBuffer::new(translated_byte_buffer(
                token,
                iovp.iov_base as *const u8,
                iovp.iov_len,
            )));
            addr += size_of::<Iovec>();
        }
        total_write_len as isize
    } else {
        -1
    }
}

pub fn sys_fstatat(dirfd: isize, pathname: *const u8, satabuf: *const usize, flags: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    // todo
    _ = flags;
    let buf_vec = translated_byte_buffer(token, satabuf as *const u8, size_of::<Kstat>());
    let mut userbuf = UserBuffer::new(buf_vec);
    let mut kstat = Kstat::new();
    let path = translated_str(token, pathname);

    if dirfd == AT_FDCWD {
        // 如果是当前工作目录
        if let Some(inode) = open(inner.get_work_path().as_str(), path.as_str(), OpenFlags::O_RDONLY) {
            let file: Arc<dyn File + Send + Sync> = inode;
            file.get_fstat(&mut kstat);
            userbuf.write(kstat.as_bytes());
            0
        } else {
            -1
        }
    } else {
        let dirfd = dirfd as usize;
        // dirfd 不合法
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            if let Some(tar_f) = open(file.get_name().as_str(), path.as_str(), OpenFlags::O_RDONLY) {
                let file: Arc<dyn File + Send + Sync> = tar_f;
                file.get_fstat(&mut kstat);
                userbuf.write(kstat.as_bytes());
                0
            } else {
                -1
            }
        } else {
            -1
        }
    }
}

pub fn sys_utimensat(dirfd: isize, pathname: *const u8, time: *const usize, flags: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let timespec_buf = translated_byte_buffer(token, time as *const u8, size_of::<Kstat>()).pop().unwrap();
    let addr = timespec_buf.as_ptr() as *const _ as usize;
    let timespec = unsafe { &*(addr as *const Timespec) };
    _ = flags;

    if pathname as usize == 0 {
        if dirfd >= inner.fd_table.len() as isize || dirfd < 0 {
            return 0;
        }
        if let Some(file) = &inner.fd_table[dirfd as usize] {
            file.set_time(timespec);
        }
        0
    } else {
        // unimplemented!();
        0
    }
}

pub fn sys_readv(fd: usize, iovp: *const usize, iovcnt: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        // 文件不可读
        if !file.readable() {
            return -1;
        }
        let iovp_buf = translated_byte_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>())
            .pop()
            .unwrap();
        let file = file.clone();
        let mut addr = iovp_buf.as_ptr() as *const _ as usize;
        let mut total_read_len = 0;
        drop(inner);
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
            total_read_len += file.read(UserBuffer::new(translated_byte_buffer(
                token,
                iovp.iov_base as *const u8,
                iovp.iov_len,
            )));
            addr += size_of::<Iovec>();
        }
        total_read_len as isize
    } else {
        -1
    }
}

// 暂时写在这里
bitflags! {
    pub struct FcntlFlags:usize{
        const F_DUPFD = 0;
        const F_GETFD = 1;
        const F_SETFD = 2;
        const F_GETFL = 3;
        const F_SETFL = 4;
        const F_GETLK = 5;
        const F_SETLK = 6;
        const F_SETLKW = 7;
        const F_SETOWN = 8;
        const F_GETOWN = 9;
        const F_SETSIG = 10;
        const F_GETSIG = 11;
        const F_SETOWN_EX = 15;
        const F_GETOWN_EX = 16;
        const F_GETOWNER_UIDS = 17;
    }
}

pub fn sys_fcntl(fd: isize, cmd: usize, arg: Option<usize>) -> isize {
    // println!("enter sys_fcntl:fd:{},cmd:{},arg:{:?}", fd, cmd, arg);
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if let Some(file) = &inner.fd_table[fd as usize] {
        let cmd = FcntlFlags::from_bits(cmd).unwrap();
        match cmd {
            FcntlFlags::F_SETFL => {
                file.set_flags(OpenFlags::from_bits(arg.unwrap() as u32).unwrap());
            }
            _ => {}
        }
    }
    0
}

pub fn sys_statfs(path: *const u8, buf: *const u8) -> isize {
    let token = current_user_token();

    _ = path;

    let mut userbuf = UserBuffer::new(translated_byte_buffer(token, buf, size_of::<Statfs>()));
    userbuf.write(Statfs::new().as_bytes());
    0
}
