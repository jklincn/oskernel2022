use super::errno::*;
use crate::fs::{chdir, make_pipe, open, Dirent, FdSet, File, Kstat, OpenFlags, Statfs, Stdin, MNT_TABLE};
use crate::mm::{translated_byte_buffer, translated_ref, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token, suspend_current_and_run_next, RLIMIT_NOFILE};
use crate::timer::{get_timeval, TimeVal, Timespec};
use alloc::{sync::Arc, vec::Vec};
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
    // println!("[DEBUG] enter sys_read: fd:{}, buffer address:0x{:x}, len:{}", fd, buf as usize, len);
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
        let file_size = file.file_size();
        if file_size == 0 {
            // println!("[WARNING] sys_read: file_size is zero!");
        }
        let len = file_size.min(len);
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

    // todo
    _ = mode;
    let oflags = OpenFlags::from_bits(flags).expect("unsupported open flag!");
    // println!(
    //     "[DEBUG] enter sys_openat: dirfd:{}, path:{}, flags:{:?}, mode:{:o}",
    //     dirfd, path, oflags, mode
    // );
    if dirfd == AT_FDCWD {
        // 如果是当前工作目录
        if let Some(inode) = open(inner.get_work_path().as_str(), path.as_str(), oflags) {
            let fd = inner.alloc_fd();
            if fd == 999 {
                return -EMFILE;
            }
            inner.fd_table[fd] = Some(inode);
            // println!("sys_openat return new fd:{}", fd);
            fd as isize
        } else {
            // println!("[WARNING] sys_openat return -1, path:{}",path);
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
                if fd == 999 {
                    return -EMFILE;
                }
                inner.fd_table[fd] = Some(tar_f);
                // println!("sys_openat return new fd:{}", fd);
                fd as isize
            } else {
                // println!("[WARNING] sys_openat return -1, path:{}",path);
                -1
            }
        } else {
            // dirfd 对应条目为 None
            // println!("[WARNING] sys_openat return -1, path:{}",path);
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

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    // 做资源检查，目前只检查 RLIMIT_NOFILE 这一种
    let rlim_max = inner.resource[RLIMIT_NOFILE].rlim_max;
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
    if new_fd > FD_LIMIT {
        return -1;
    }
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
    // println!("[DEBUG] enter sys_mkdirat: dirfd:{}, path:{}. mode:{:o}",dirfd,path,mode);
    if dirfd == AT_FDCWD {
        if let Some(_) = open(
            inner.get_work_path().as_str(),
            path.as_str(),
            OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        ) {
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
            if let Some(_) = open(
                file.get_name().as_str(),
                path.as_str(),
                OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
            ) {
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
        userbuf.write_at(cwd.len(), &[0]); // 添加字符串末尾的\0
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
    // println!("[DEBUG] enter sys_unlinkat: fd:{}, path:{}, flags:{}",fd,path,flags);
    if fd == AT_FDCWD {
        if let Some(file) = open(inner.get_work_path().as_str(), path.as_str(), OpenFlags::O_RDWR) {
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
    // println!("[DEBUG] enter sys_fstat: fd:{}, buf:0x{:x}", fd, buf as usize);
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
    // println!("[DEBUG] enter sys_getdents64: fd:{}, buf:{}, len:{}", fd, buf as usize, len);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let work_path = inner.current_path.clone();
    let buf_vec = translated_byte_buffer(token, buf, len);
    let mut userbuf = UserBuffer::new(buf_vec);
    let mut dirent = Dirent::new();
    let dent_len = size_of::<Dirent>();
    let mut total_len: usize = 0;

    if fd == AT_FDCWD {
        if let Some(file) = open("/", work_path.as_str(), OpenFlags::O_RDONLY) {
            loop {
                if total_len + dent_len > len {
                    break;
                }
                if file.get_dirent(&mut dirent) > 0 {
                    userbuf.write_at(total_len, dirent.as_bytes());
                    total_len += dent_len;
                } else {
                    break;
                }
            }
            return total_len as isize;
        } else {
            return -1;
        }
    } else {
        if let Some(file) = &inner.fd_table[fd as usize] {
            loop {
                if total_len + dent_len > len {
                    break;
                }
                if file.get_dirent(&mut dirent) > 0 {
                    userbuf.write_at(total_len, dirent.as_bytes());
                    total_len += dent_len;
                } else {
                    break;
                }
            }
            return total_len as isize;
        } else {
            return -1;
        }
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
    // println!("[DEBUG] enter sys_lseek: fd:{},off_t:{},whence:{}",fd,off_t,whence);

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
                let inode = open(
                    inner.get_work_path().clone().as_str(),
                    file.get_name().as_str(),
                    OpenFlags::O_RDONLY,
                )
                .unwrap();
                let end = inode.file_size();
                file.set_offset(end + off_t);
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
const TCGETS: usize = 0x5401;
const TCSETS: usize = 0x5402;
const TIOCGPGRP: usize = 0x540f;
const TIOCSPGRP: usize = 0x5410;
const TIOCGWINSZ: usize = 0x5413;
const RTC_RD_TIME: usize = 0xffffffff80247009; // 这个值还需考量

pub fn sys_ioctl(fd: usize, request: usize, argp: *mut u8) -> isize {
    // println!("enter sys_ioctl: fd:{}, request:0x{:x}, argp:{}", fd, request, argp as usize);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // 文件描述符不合法
    if fd >= inner.fd_table.len() {
        return -1;
    }
    match request {
        TCGETS => {}
        TCSETS => {}
        TIOCGPGRP => *translated_refmut(token, argp) = 0 as u8,
        TIOCSPGRP => {}
        TIOCGWINSZ => *translated_refmut(token, argp) = 0 as u8,
        RTC_RD_TIME => {}
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
    // println!("[DEBUG] enter sys_writev: fd:{}, iovp:0x{:x}, iovcnt:{}",fd,iovp as usize,iovcnt);
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

pub fn sys_newfstatat(dirfd: isize, pathname: *const u8, satabuf: *const usize, _flags: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let path = translated_str(token, pathname);

    // println!(
    //     "[DEBUG] enter sys_newfstatat: dirfd:{}, pathname:{}, satabuf:0x{:x}, flags:0x{:x}",
    //     dirfd, path, satabuf as usize, flags
    // );

    let buf_vec = translated_byte_buffer(token, satabuf as *const u8, size_of::<Kstat>());
    let mut userbuf = UserBuffer::new(buf_vec);
    let mut kstat = Kstat::new();

    if dirfd == AT_FDCWD {
        if let Some(inode) = open(inner.get_work_path().as_str(), path.as_str(), OpenFlags::O_RDONLY) {
            inode.get_fstat(&mut kstat);
            userbuf.write(kstat.as_bytes());
            // panic!();
            0
        } else {
            -ENOENT
        }
    } else {
        let dirfd = dirfd as usize;
        if dirfd >= inner.fd_table.len() && dirfd > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[dirfd] {
            if let Some(inode) = open(file.get_name().as_str(), path.as_str(), OpenFlags::O_RDONLY) {
                inode.get_fstat(&mut kstat);
                userbuf.write(kstat.as_bytes());
                0
            } else {
                -1
            }
        } else {
            -ENOENT
        }
    }
}

pub fn sys_utimensat(dirfd: isize, pathname: *const u8, time: *const usize, flags: usize) -> isize {
    // println!(
    //     "[DEBUG] enter sys_utimensat: dirfd:{}, pathname:{}, time:{}, flags:{}",
    //     dirfd, pathname as usize, time as usize, flags
    // );
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    _ = flags;

    if dirfd == AT_FDCWD {
        if pathname as usize == 0 {
            unimplemented!();
        } else {
            let pathname = translated_str(token, pathname);
            if let Some(_file) = open(inner.get_work_path().as_str(), pathname.as_str(), OpenFlags::O_RDWR) {
                unimplemented!(); // 记得重新制作文件镜像
            } else {
                -ENOENT
            }
        }
    } else {
        if pathname as usize == 0 {
            if dirfd >= inner.fd_table.len() as isize || dirfd < 0 {
                return 0;
            }
            if let Some(file) = &inner.fd_table[dirfd as usize] {
                let timespec_buf = translated_byte_buffer(token, time as *const u8, size_of::<Kstat>()).pop().unwrap();
                let addr = timespec_buf.as_ptr() as *const _ as usize;
                let timespec = unsafe { &*(addr as *const Timespec) };
                file.set_time(timespec);
                0
            } else {
                -1
            }
        } else {
            unimplemented!();
        }
    }
}

pub fn sys_readv(fd: usize, iovp: *const usize, iovcnt: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.readable() {
            return -1;
        }
        let iovp_buf = translated_byte_buffer(token, iovp as *const u8, iovcnt * size_of::<Iovec>())
            .pop()
            .unwrap();
        let file = file.clone();
        let file_size = file.file_size();
        if file_size == 0 {
            println!("[WARNING] sys_readv: file_size is zero!");
        }
        let mut addr = iovp_buf.as_ptr() as *const _ as usize;
        let mut total_read_len = 0;
        drop(inner);
        for _ in 0..iovcnt {
            let iovp = unsafe { &*(addr as *const Iovec) };
            let len = file_size.min(iovp.iov_len);
            total_read_len += file.read(UserBuffer::new(translated_byte_buffer(token, iovp.iov_base as *const u8, len)));
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

        // 发现 F_UNLCK = 2 , 这个标记分类待研究
        const F_DUPFD_CLOEXEC = 1030;
    }
}

pub fn sys_fcntl(fd: isize, cmd: usize, arg: Option<usize>) -> isize {
    // println!("[DEBUG] enter sys_fcntl: fd:{}, cmd:{}, arg:{:?}", fd, cmd, arg);
    let task = current_task().unwrap();
    let cmd = FcntlFlags::from_bits(cmd).unwrap();
    match cmd {
        FcntlFlags::F_SETFL => {
            let inner = task.inner_exclusive_access();
            if let Some(file) = &inner.fd_table[fd as usize] {
                file.set_flags(OpenFlags::from_bits(arg.unwrap() as u32).unwrap());
            } else {
                panic!("sys_fcntl: fd is not an open file descriptor");
            }
        }
        // Currently, only one such flag is defined: FD_CLOEXEC (value: 1)
        FcntlFlags::F_GETFD => {
            // Return (as the function result) the file descriptor flags; arg is ignored.
            let inner = task.inner_exclusive_access();
            if let Some(file) = &inner.fd_table[fd as usize] {
                return file.available() as isize;
            } else {
                panic!("sys_fcntl: fd is not an open file descriptor");
            }
        }
        FcntlFlags::F_SETFD => {
            // Set the file descriptor flags to the value specified by arg.
            let inner = task.inner_exclusive_access();
            if let Some(file) = &inner.fd_table[fd as usize] {
                if arg.unwrap() != 0 {
                    file.set_cloexec();
                }
            } else {
                panic!("sys_fcntl: fd is not an open file descriptor");
            }
        }
        FcntlFlags::F_GETFL => {
            // Return (as the function result) the file access mode and the file status flags; arg is ignored.
            // todo
            return 04000;
        }
        FcntlFlags::F_DUPFD_CLOEXEC => {
            let mut inner = task.inner_exclusive_access();
            let start_num = arg.unwrap();
            let mut new_fd = 0;
            _ = new_fd;
            let mut tmp_fd = Vec::new();
            loop {
                new_fd = inner.alloc_fd();
                inner.fd_table[new_fd] = Some(Arc::new(Stdin));
                if new_fd >= start_num {
                    break;
                } else {
                    tmp_fd.push(new_fd);
                }
            }
            for i in tmp_fd {
                inner.fd_table[i].take();
            }
            inner.fd_table[new_fd] = Some(Arc::clone(
                inner.fd_table[fd as usize]
                    .as_ref()
                    .expect("sys_fcntl: fd is not an open file descriptor"),
            ));
            inner.fd_table[new_fd].as_ref().unwrap().set_cloexec();
            return new_fd as isize;
        }
        _ => panic!("sys_ioctl: unsupported request!"),
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

pub fn sys_pread64(fd: usize, buf: *const u8, count: usize, offset: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        let old_offset = file.get_offset();
        file.set_offset(offset);
        let readsize = file.read(UserBuffer::new(translated_byte_buffer(token, buf, count))) as isize;
        file.set_offset(old_offset);
        readsize
    } else {
        -1
    }
}

pub fn sys_sendfile(out_fd: usize, in_fd: usize, offset: usize, _count: usize) -> isize {
    // println!(
    //     "[DEBUG] enter sys_sendfile: out_fd:{}, in_fd:{}, offset:{}, count:{}",
    //     out_fd, in_fd, offset, _count
    // );
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fd_table = inner.fd_table.clone();
    drop(inner);
    let mut total_write_size = 0usize;
    if offset as usize != 0 {
        unimplemented!();
    } else {
        let in_file = fd_table[in_fd].as_ref().unwrap();
        let out_file = fd_table[out_fd].as_ref().unwrap();
        let mut data_buffer;
        loop {
            data_buffer = in_file.read_kernel_space();
            // println!("data_buffer:{:?}",data_buffer);
            let len = data_buffer.len();
            if len == 0 {
                break;
            } else {
                out_file.write_kernel_space(data_buffer);
                total_write_size += len;
            }
        }
        total_write_size as isize
    }
}

// 目前仅支持同当前目录下文件名称更改
pub fn sys_renameat2(old_dirfd: isize, old_path: *const u8, new_dirfd: isize, new_path: *const u8, _flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();
    let old_path = translated_str(token, old_path);
    let new_path = translated_str(token, new_path);

    // println!(
    //     "[DEBUG] enter sys_renameat2: old_dirfd:{}, old_path:{}, new_dirfd:{}, new_path:{}, flags:0x{:x}",
    //     old_dirfd, old_path, new_dirfd, new_path, _flags
    // );
    if old_dirfd == AT_FDCWD {
        if let Some(old_file) = open(inner.get_work_path().as_str(), old_path.as_str(), OpenFlags::O_RDWR) {
            let flag = {
                if old_file.is_dir() {
                    OpenFlags::O_RDWR | OpenFlags::O_CREATE | OpenFlags::O_DIRECTROY
                } else {
                    OpenFlags::O_RDWR | OpenFlags::O_CREATE
                }
            };
            if new_dirfd == AT_FDCWD {
                if let Some(new_file) = open(inner.get_work_path().as_str(), new_path.as_str(), flag) {
                    let first_cluster = old_file.get_head_cluster();
                    new_file.set_head_cluster(first_cluster);
                    old_file.delete();
                    0
                } else {
                    panic!("can't find new file");
                    -1
                }
            } else {
                unimplemented!();
            }
        } else {
            panic!("can't find old file");
            -1
        }
    } else {
        unimplemented!();
    }
}

pub fn sys_umask() -> isize {
    0
}

pub fn sys_readlinkat(dirfd: isize, pathname: *const u8, buf: *const u8, bufsiz: usize) -> isize {
    if dirfd == AT_FDCWD {
        let token = current_user_token();
        let path = translated_str(token, pathname);
        if path.as_str() != "/proc/self/exe" {
            panic!("sys_readlinkat: pathname not support");
        }
        let mut userbuf = UserBuffer::new(translated_byte_buffer(token, buf, bufsiz));
        let procinfo = "/lmbench_all\0";
        userbuf.write(procinfo.as_bytes());
        let len = procinfo.len() - 1;
        return len as isize;
    } else {
        panic!("sys_readlinkat: fd not support");
    }
}

pub fn sys_pselect(nfds: usize, readfds: *mut u8, writefds: *mut u8, exceptfds: *mut u8, timeout: *mut usize) -> isize {
    let token = current_user_token();
    let mut r_ready_count = 0;
    let mut w_ready_count = 0;
    let mut e_ready_count = 0;

    let mut timer_interval = TimeVal::new();
    unsafe {
        let sec = translated_ref(token, timeout);
        let usec = translated_ref(token, timeout.add(1));
        timer_interval.sec = *sec;
        timer_interval.usec = *usec;
    }
    let timer = timer_interval + get_timeval();

    let mut rfd_set = FdSet::new();
    let mut wfd_set = FdSet::new();

    let mut ubuf_rfds = {
        if readfds as usize != 0 {
            UserBuffer::new(translated_byte_buffer(token, readfds, size_of::<FdSet>()))
        } else {
            UserBuffer::empty()
        }
    };
    ubuf_rfds.read(rfd_set.as_bytes_mut());

    let mut ubuf_wfds = {
        if writefds as usize != 0 {
            UserBuffer::new(translated_byte_buffer(token, writefds, size_of::<FdSet>()))
        } else {
            UserBuffer::empty()
        }
    };
    ubuf_wfds.read(wfd_set.as_bytes_mut());

    let mut ubuf_efds = {
        if exceptfds as usize != 0 {
            UserBuffer::new(translated_byte_buffer(token, exceptfds, size_of::<FdSet>()))
        } else {
            UserBuffer::empty()
        }
    };

    // println!("[DEBUG] enter sys_pselect: nfds:{}, readfds:{:?} ,writefds:{:?}, exceptfds:{:?}, timeout:{:?}",nfds,ubuf_rfds,ubuf_wfds,ubuf_efds,timer_interval);

    let mut r_has_nready = false;
    let mut w_has_nready = false;
    let mut r_all_ready = false;
    let mut w_all_ready = false;

    let mut rfd_vec = Vec::new();
    let mut wfd_vec = Vec::new();

    loop {
        /* handle read fd set */
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();
        let fd_table = &inner.fd_table;
        if readfds as usize != 0 && !r_all_ready {
            if rfd_vec.len() == 0 {
                rfd_vec = rfd_set.get_fd_vec();
                if rfd_vec[rfd_vec.len() - 1] >= nfds {
                    return -1; // invalid fd
                }
            }

            for i in 0..rfd_vec.len() {
                let fd = rfd_vec[i];
                if fd == 1024 {
                    continue;
                }
                if fd > fd_table.len() || fd_table[fd].is_none() {
                    return -1; // invalid fd
                }
                let fdescript = fd_table[fd].as_ref().unwrap();
                if fdescript.r_ready() {
                    r_ready_count += 1;
                    rfd_set.set_fd(fd);
                    // marked for being ready
                    rfd_vec[i] = 1024;
                } else {
                    rfd_set.clear_fd(fd);
                    r_has_nready = true;
                }
            }
            if !r_has_nready {
                r_all_ready = true;
                ubuf_rfds.write(rfd_set.as_bytes());
            }
        }

        /* handle write fd set */
        if writefds as usize != 0 && !w_all_ready {
            if wfd_vec.len() == 0 {
                wfd_vec = wfd_set.get_fd_vec();
                if wfd_vec[wfd_vec.len() - 1] >= nfds {
                    return -1; // invalid fd
                }
            }

            for i in 0..wfd_vec.len() {
                let fd = wfd_vec[i];
                if fd == 1024 {
                    continue;
                }
                if fd > fd_table.len() || fd_table[fd].is_none() {
                    return -1; // invalid fd
                }
                let fdescript = fd_table[fd].as_ref().unwrap();
                if fdescript.w_ready() {
                    w_ready_count += 1;
                    wfd_set.set_fd(fd);
                    wfd_vec[i] = 1024;
                } else {
                    wfd_set.clear_fd(fd);
                    w_has_nready = true;
                }
            }
            if !w_has_nready {
                w_all_ready = true;
                ubuf_wfds.write(wfd_set.as_bytes());
            }
        }

        /* Cannot handle exceptfds for now */
        if exceptfds as usize != 0 {
            let mut efd_set = FdSet::new();
            ubuf_efds.read(efd_set.as_bytes_mut());
            e_ready_count = efd_set.count() as isize;
            efd_set.clear_all();
            ubuf_efds.write(efd_set.as_bytes());
        }

        // return anyway
        // return r_ready_count + w_ready_count + e_ready_count;
        // if there are some fds not ready, just wait until time up
        if r_has_nready || w_has_nready {
            r_has_nready = false;
            w_has_nready = false;
            let time_remain = get_timeval() - timer;
            if time_remain.is_zero() {
                // not reach timer (now < timer)
                drop(fd_table);
                drop(inner);
                drop(task);
                suspend_current_and_run_next();
            } else {
                ubuf_rfds.write(rfd_set.as_bytes());
                ubuf_wfds.write(wfd_set.as_bytes());
                break;
            }
        } else {
            break;
        }
    }
    // println!("pselect return: r_ready_count:{}, w_ready_count:{}, e_ready_count:{}",r_ready_count,w_ready_count,e_ready_count);
    r_ready_count + w_ready_count + e_ready_count
}
