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
use crate::fs::{make_pipe, open, DiskInodeType, OpenFlags, MNT_TABLE,ch_dir};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use alloc::{sync::Arc, vec::Vec, string::String};

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
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
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
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// ### 打开文件函数
/// - `path`：文件路径
/// - `flags`：打开文件权限标志
/// - 返回值
///     - 成功打开，返回文件标识符
///     - 打开失败，返回 -1
// pub fn sys_open(path: *const u8, flags: u32) -> isize {
//     let task = current_task().unwrap();
//     let token = current_user_token();
//     let path = translated_str(token, path);

//     if let Some(inode) = open(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
//         let mut inner = task.inner_exclusive_access();
//         let fd = inner.alloc_fd();
//         inner.fd_table[fd] = Some(inode);
//         fd as isize
//     } else {
//         -1
//     }
// }

pub fn sys_openat(dirfd: isize, path: *const u8, flags: u32, mode: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    // 这里传入的地址为用户的虚地址，因此要使用用户的虚地址进行映射
    let path = translated_str(token, path);

    let mut inner = task.inner_exclusive_access();
    // println!("dirfd:{},path:{},flags:{},mode:{}",dirfd,path,flags,mode);
    let oflags = OpenFlags::from_bits(flags).unwrap();
    if dirfd == AT_FDCWD {
        if let Some(inode) = open(inner.get_work_path().as_str(), path.as_str(), oflags, DiskInodeType::File) {
            let fd = inner.alloc_fd();
            inner.fd_table[fd] = Some(inode);
            fd as isize
        } else {
            -1
        }
    } else {
        let fd_usz = dirfd as usize;
        if fd_usz >= inner.fd_table.len() && fd_usz > FD_LIMIT {
            return -1;
        }

        if let Some(file) = &inner.fd_table[fd_usz] {
            // 需要新建文件
            if oflags.contains(OpenFlags::O_CREATE) {
                if let Some(tar_f) = file.create(path.as_str(), DiskInodeType::File) {
                    let fd = inner.alloc_fd();
                    inner.fd_table[fd] = Some(tar_f);
                    return fd as isize;
                } else {
                    return -1;
                }
            }

            // 需要新建目录
            if oflags.contains(OpenFlags::O_DIRECTROY) {
                if let Some(tar_f) = file.create(path.as_str(), DiskInodeType::Directory) {
                    let fd = inner.alloc_fd();
                    inner.fd_table[fd] = Some(tar_f);
                    return fd as isize;
                } else {
                    return -1;
                }
            }

            // 正常打开文件
            if let Some(tar_f) = file.find(path.as_str(), oflags) {
                let fd = inner.alloc_fd();
                inner.fd_table[fd] = Some(tar_f);
                fd as isize
            } else {
                return -1;
            }
        } else {
            return -1;
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
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe) = read_fd as u32;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd as u32;
    0
}

/// ### 将进程中一个已经打开的文件复制一份并分配到一个新的文件描述符中。
/// - 参数：fd 表示进程中一个已经打开的文件的文件描述符。
/// - 返回值：
///     - 能够访问已打开文件的新文件描述符。
///     - 如果出现了错误则返回 -1，可能的错误原因是：传入的 fd 并不对应一个合法的已打开文件。
/// - syscall ID：24
pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
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

pub fn sys_mkdirat(dirfd: isize, path: *const u8, mode: u32) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let path = translated_str(token, path);
    if dirfd == AT_FDCWD {
        if let Some(_) = open(
            inner.get_work_path().as_str(),
            path.as_str(),
            OpenFlags::O_CREATE,
            DiskInodeType::Directory,
        ) {
            return 0;
        } else {
            return -1;
        }
    } else {
        // DEBUG: 获取dirfd的OSInode
        let fd_usz = dirfd as usize;
        if fd_usz >= inner.fd_table.len() && fd_usz > FD_LIMIT {
            return -1;
        }
        if let Some(file) = &inner.fd_table[fd_usz] {
            if let Some(_) = file.create(path.as_str(), DiskInodeType::Directory) {
                return 0;
            } else {
                return -1;
            }
        } else {
            return -1;
        }
    }
}

// buf：用于保存当前工作目录的字符串。当buf设为NULL，由系统来分配缓存区

pub fn sys_getcwd(buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let buf_vec = translated_byte_buffer(token, buf, len);
    let inner = task.inner_exclusive_access();

    if buf as usize == 0 {
        unimplemented!();
    } else {
        let mut userbuf = UserBuffer::new(buf_vec);
        let cwd = inner.current_path.as_bytes();
        userbuf.write(cwd);
        return buf as isize;
    }
}

pub fn sys_mount(p_special: *const u8, p_dir: *const u8, p_fstype: *const u8, flags: usize, data: *const u8) -> isize {
    // TODO
    let token = current_user_token();
    let special = translated_str(token, p_special);
    let dir = translated_str(token, p_dir);
    let fstype = translated_str(token, p_fstype);
    MNT_TABLE.lock().mount(special, dir, fstype, flags as u32)
}

pub fn sys_umount(p_special: *const u8, flags: usize) -> isize {
    // TODO
    let token = current_user_token();
    let special = translated_str(token, p_special);
    MNT_TABLE.lock().umount(special, flags as u32)
}

pub fn sys_unlinkat(fd: isize, path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    // 这里传入的地址为用户的虚地址，因此要使用用户的虚地址进行映射
    let path = translated_str(token, path);
    let inner = task.inner_exclusive_access();

    if fd == AT_FDCWD {
        if let Some(file) = open(
            inner.get_work_path().as_str(),
            path.as_str(),
            OpenFlags::from_bits(0).unwrap(),
            DiskInodeType::File,
        ) {
            file.delete();
            0
        } else {
            -1
        }
    } else {
        unimplemented!();
    }
}

pub fn sys_getdents64(fd:isize, buf: *mut u8, len:usize)->isize{
    unimplemented!();
}

pub fn sys_chdir(path: *const u8) -> isize{
    //print_core_info();
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let path = translated_str(token, path);
    let mut work_path = inner.current_path.clone();
    //println!("work path = {}", work_path);
    //println!("path  = {}, len = {}", path, path.len());
    //println!("curr inode id = {}", curr_inode_id);
    let new_ino_id = ch_dir(work_path.as_str(), path.as_str()) as isize;
    //println!("new inode id = {}", new_ino_id);
    if new_ino_id >= 0 {
        //inner.current_inode = new_ino_id as u32;
        if path.chars().nth(0).unwrap() == '/' {
            inner.current_path = path.clone();
        } else {
            work_path.push('/');
            work_path.push_str(path.as_str());
            let mut path_vec: Vec<&str> = work_path.as_str().split('/').collect();
            let mut new_pathv: Vec<&str> = Vec::new(); 
            for i in 0..path_vec.len(){
                if path_vec[i] == "" || path_vec[i] == "." {
                    continue;
                }
                if path_vec[i] == ".." {
                    new_pathv.pop();
                    continue;
                } 
                new_pathv.push(path_vec[i]);
            }
            let mut new_wpath = String::new();
            for i in 0..new_pathv.len(){
                new_wpath.push('/');
                new_wpath.push_str(new_pathv[i]);
            }
            if new_pathv.len() == 0 {
                new_wpath.push('/');
            }
            //println!("after cd workpath = {}", new_wpath);
            inner.current_path = new_wpath.clone();
        }
        new_ino_id
    }else{
        new_ino_id
    }
}