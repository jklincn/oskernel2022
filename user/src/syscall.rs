/// # 系统调用模块
/// `user/src/syscall.rs`
/// ## 可用实现函数
/// ```
/// pub fn sys_dup(fd: usize) -> isize
/// pub fn sys_open(path: &str, flags: u32) -> isize
/// pub fn sys_close(fd: usize) -> isize
/// pub fn sys_pipe(pipe: &mut [usize]) -> isize
/// pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize
/// pub fn sys_write(fd: usize, buffer: &[u8]) -> isize
/// pub fn sys_exit(exit_code: i32) -> isize
/// pub fn sys_yield() -> isize
/// pub fn sys_kill(pid: usize, signal: i32) -> isize
/// pub fn sys_get_time() -> isize
/// pub fn sys_getpid() -> isize
/// pub fn sys_fork() -> isize
/// pub fn sys_exec(path: &str) -> isize
/// pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize
/// ```
//

use core::arch::asm;

const SYSCALL_DUP:      usize = 24;
const SYSCALL_OPEN:     usize = 56;
const SYSCALL_CLOSE:    usize = 57;
const SYSCALL_PIPE:     usize = 59;
const SYSCALL_READ:     usize = 63;
const SYSCALL_WRITE:    usize = 64;
const SYSCALL_EXIT:     usize = 93;
const SYSCALL_YIELD:    usize = 124;
const SYSCALL_KILL:     usize = 129;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID:   usize = 172;
const SYSCALL_FORK:     usize = 220;
const SYSCALL_EXEC:     usize = 221;
const SYSCALL_WAITPID:  usize = 260;

/// ### 汇编完成的系统调用
/// - id : 系统调用 ID
/// - args : 系统调用参数
/// - ret : 系统调用返回值
/// 
/// 通过 `ecall` 调用批处理系统提供的接口，由于应用程序运行在用户态（即 U模式）， `ecall` 指令会触发 名为 `Environment call from U-mode` 的异常，并 `Trap` 进入 S模式执行**批处理系统**针对这个异常特别**提供**的服务代码
/// 
/// 我们知道系统调用实际上是汇编指令级的二进制接口，因此这里给出的只是使用 Rust 语言描述的 API 版本。在实际调用的时候，我们需要按照 RISC-V 调用规范（即ABI格式）**在合适的寄存器中放置系统调用的参数**，然后执行 ecall 指令触发 Trap。在 Trap 回到 U 模式的应用程序代码之后，会从 ecall 的下一条指令继续执行，同时我们能够按照调用规范在合适的寄存器中读取返回值。
/// 
/// RISC-V 调用规范:
/// - `a0~a6` 保存系统调用的参数
/// - `a0~a1` 保存系统调用的返回值
/// - `a7` 用来传递 syscall ID
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(   // 相比 global_asm! ，asm! 宏可以获取上下文中的变量信息并允许嵌入的汇编代码对这些变量进行操作
            "ecall",
            // a0 寄存器，它同时作为输入和输出, 在行末的变量部分使用 {in_var} => {out_var} 分别表示上下文中的输入变量和输出变量
            inlateout("x10") args[0] => ret,
            in("x11") args[1],  // 在编译器的帮助下完成变量到寄存器的绑定
            in("x12") args[2],      // 有些时候不必将变量绑定到固定的寄存器,
            in("x17") id            // 此时 asm! 宏可以自动完成寄存器分配
        );
    }
    ret
}

/// ### 将进程中一个已经打开的文件复制一份并分配到一个新的文件描述符中。
/// - 参数：fd 表示进程中一个已经打开的文件的文件描述符。
/// - 返回值：
///     - 能够访问已打开文件的新文件描述符。
///     - 如果出现了错误则返回 -1，可能的错误原因是：传入的 fd 并不对应一个合法的已打开文件。
/// - syscall ID：24
pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

/// ### 打开一个常规文件，并返回可以访问它的文件描述符。
/// |参数|描述|
/// |--|--|
/// |`path`|描述要打开的文件的文件名|
/// |`flags`|描述打开文件的标志|
/// 
/// 返回值：如果出现了错误则返回 -1，否则返回打开常规文件的文件描述符。可能的错误原因是：文件不存在。
/// syscall ID：56
pub fn sys_open(path: &str, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

/// ### 当前进程关闭一个文件。
/// - fd：要关闭的文件的文件描述符。
/// - 返回值：如果成功关闭则返回 0 ，否则返回 -1 。
///     - 可能的出错原因：传入的文件描述符并不对应一个打开的文件。
pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}
/// ### 为当前进程打开一个管道。
/// - `pipe` 表示应用地址空间中的一个长度为 `2` 的 `usize` 数组的起始地址，
/// 内核需要按顺序将管道读端和写端的文件描述符写入到数组中。
/// - 返回值：如果出现了错误则返回 -1，否则返回 0 。可能的错误原因是：传入的地址不合法。
/// - syscall ID：59
pub fn sys_pipe(pipe: &mut [usize]) -> isize {
    syscall(SYSCALL_PIPE, [pipe.as_mut_ptr() as usize, 0, 0])
}

/// ### 从文件中读取一段内容到缓冲区
/// - 参数
///     - `fd` 表示待读取文件的文件描述符；
///     - `buffer` 表示内存中缓冲区的起始地址；
/// - 返回值：如果出现了错误则返回 -1，否则返回实际读到的字节数。
/// - syscall ID：63
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

/// ### 将内存中缓冲区中的数据写入**文件**。
/// - 参数：
///     - `fd` 表示待写入文件的文件描述符；
///     - `buf` 表示内存中缓冲区的起始地址；
///     - `len` 表示内存中缓冲区的长度。
/// - 返回值：返回成功写入的长度。
/// - syscall ID：64
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    // 注意 sys_write 使用一个 &[u8] 切片类型来描述缓冲区，这是一个 胖指针 (Fat Pointer)，
    // 里面既包含缓冲区的起始地址，还 包含缓冲区的长度。我们可以分别通过 as_ptr 和 len 方法取出它们并独立地作为实际的系统调用参数。
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

/// ### 退出应用程序并将返回值告知批处理系统。
/// - 参数：`xstate` 表示应用程序的返回值。
/// - 返回值：该系统调用不应该返回。
/// - syscall ID：93
pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

/// ### 通过系统调用放弃CPU资源
/// - 无参数
/// - 返回值总是0
/// - syscall ID：124
pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn sys_kill(pid: usize, signal: i32) -> isize {
    syscall(SYSCALL_KILL, [pid, signal as usize, 0])
}

/// ### 通过系统调用获取CPU上电时间
/// - 无参数
/// - 返回值：CPU上电时间
/// - syscall ID：169
pub fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

/// 获取当前正在运行程序的 PID
pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

/// ### 当前进程 fork 出来一个子进程。
/// - 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID 。
/// - syscall ID：220
pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

/// ### 通过系统调用执行新的程序
/// - 参数 
///     - `path` 给出了要加载的可执行文件的名字，必须在最后加 `\0`
///     - `args` 数组中的每个元素都是一个命令行参数字符串的起始地址，以地址为0表示参数尾
/// - 返回值：如果出错的话（如找不到名字相符的可执行文件）则返回 -1，否则不应该返回。
/// - syscall ID：221
pub fn sys_exec(path: &str, args: &[*const u8]) -> isize {
    // path 作为 &str 类型是一个胖指针，既有起始地址又包含长度信息。
    // 在实际进行系统调用的时候，我们只会将起始地址传给内核
    // 这就需要应用负责在传入的字符串的末尾加上一个 \0 ，这样内核才能知道字符串的长度
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, args.as_ptr() as usize, 0])
}

/// ### 当前进程等待一个子进程变为僵尸进程，回收其全部资源并收集其返回值。
/// - 参数：
///     - pid 表示要等待的子进程的进程 ID，如果为 -1 的话表示等待任意一个子进程；
///     - exit_code 表示保存子进程返回值的地址，如果这个地址为 0 的话表示不必保存。
/// - 返回值：
///     - 如果要等待的子进程不存在则返回 -1；
///     - 否则如果要等待的子进程均未结束则返回 -2；
///     - 否则返回结束的子进程的进程 ID。
/// - syscall ID：260
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}
