/// # 文件读写模块
/// `os/src/syscall/fs.rs`
/// ## 实现功能
/// ```
/// pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize
/// ```
//

const FD_STDOUT: usize = 1;

/// ### 写文件函数
/// - `fd` 表示待写入文件的文件描述符；
/// - `buf` 表示内存中缓冲区的起始地址；
/// - `len` 表示内存中缓冲区的长度。
/// - 返回值：成功写入的长度。
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);  // 这里我们并没有检查传入参数的安全性，即使会在出错严重的时候 panic，还是会存在安全隐患
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
