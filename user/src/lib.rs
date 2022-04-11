// user/src/lib.rs
// 用户模式下程序的主文件

#![no_std]
#![feature(linkage)]    // 为了支持软链接操作而加入
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_items;

extern crate alloc;
#[macro_use]
extern crate bitflags;

use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;

const USER_HEAP_SIZE: usize = 16384;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    unsafe {    // 初始化一个由伙伴系统控制的堆空间
        HEAP.lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    // 将起始地址转换为对应地址下的参数向量
    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    // 调用main函数得到一个类型为i32的返回值
    // 最后调用用户库提供的 exit 接口退出应用程序
    // 并将 main 函数的返回值告知批处理系统
    exit(main(argc, v.as_slice()));   
    panic!("unreachable after sys_exit!");
}

// 我们使用 Rust 的宏将其函数符号 main 标志为弱链接。
// 这样在最后链接的时候，虽然在 lib.rs 和 bin 目录下的某个应用程序
// 都有 main 符号，但由于 lib.rs 中的 main 符号是弱链接，
// 链接器会使用 bin 目录下的应用主逻辑作为 main 。
// 这里我们主要是进行某种程度上的保护，如果在 bin 目录下找不到任何 main ，
// 那么编译也能够通过，但会在运行时报错。
#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR   = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC  = 1 << 10;
    }
}

mod syscall;
use syscall::*;


pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}
/// ### 创建一个管道
/// - `pipe_fd`：管道读/写端的文件米描述符数组(大小为2)的起始地址
pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}
/// ### 从文件读取数据到缓冲区
/// - `fd` : 文件描述符(aka.文件描述符表下标)
///     - `0`：stdin
/// - `buf`: 缓冲区起始地址
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}
/// ### 将缓冲区数据写到文件
/// - `fd` : 文件描述符(aka.文件描述符表下标)
///     - `1`：stdout
///     - `2`：stderr
/// - `buf`: 缓冲区起始地址
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}
pub fn getpid() -> isize {
    sys_getpid()
}
/// ### 系统调用 `sys_fork` 的封装
/// - 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID
pub fn fork() -> isize {
    sys_fork()
}
/// ### 系统调用 `sys_exec` 的封装
/// - `path`：程序路径，必须在最后加 \0
/// - `args`：参数数组，数组中的每个元素都是一个命令行参数字符串的起始地址
pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}
/// ### 当前进程等待一个子进程变为僵尸进程，回收其全部资源并收集其返回值
/// - 返回值：
///     - 如果要等待的子进程不存在则返回 -1；
///     - 否则如果要等待的子进程均未结束则返回 -2；
///     - 否则返回结束的子进程的进程 ID
pub fn wait(exit_code: &mut i32) -> isize {
    loop {  // 循环检查，后期会修改为阻塞的
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => { //要等待的子进程存在但它却尚未退出
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}
/// 等待一个进程标识符的值为 `pid` 的子进程结束
pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {  // 循环检查，后期会修改为阻塞的
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => { // 要等待的子进程存在但它却尚未退出
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}
/// 判断一个进程是否退出
pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _)
}
/// 通过 `sys_yield` 放弃CPU一段时间
pub fn sleep(period_ms: usize) {
    let start = sys_get_time();
    while sys_get_time() < start + period_ms as isize {
        sys_yield();
    }
}
