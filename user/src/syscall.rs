/// # 系统调用模块
/// ## 可用实现函数
/// ```
/// pub fn sys_write(fd: usize, buffer: &[u8]) -> isize
/// pub fn sys_exit(exit_code: i32) -> isize
/// pub fn sys_yield() -> isize
/// pub fn sys_get_time() -> isize
/// ```
//

use core::arch::asm;

const SYSCALL_WRITE:    usize = 64;
const SYSCALL_EXIT:     usize = 93;
const SYSCALL_YIELD:    usize = 124;
const SYSCALL_GET_TIME: usize = 169;

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

/// - 功能：将内存中缓冲区中的数据写入**文件**。
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

/// - 功能：退出应用程序并将返回值告知批处理系统。
/// - 参数：`xstate` 表示应用程序的返回值。
/// - 返回值：该系统调用不应该返回。
/// - syscall ID：93
pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

/// - 通过系统调用放弃CPU资源
/// - 无参数
/// - 返回值总是0
/// - syscall ID：124
pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

/// - 通过系统调用获取CPU上电时间
/// - 无参数
/// - 返回值：CPU上电时间
/// - syscall ID：169
pub fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}