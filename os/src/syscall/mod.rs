/// # 系统调用模块
/// `os/src/syscall/mod.rs`


const SYSCALL_GETCWD:   usize = 17;
const SYSCALL_DUP:      usize = 23;
const SYSCALL_DUP3:     usize = 24;
const SYSCALL_FCNTL:    usize = 25;
const SYSCALL_IOCTL:    usize = 29;
const SYSCALL_MKDIRAT:  usize = 34;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_UMOUNT2:  usize = 39;
const SYSCALL_MOUNT:    usize = 40;
const SYSCALL_STATFS:   usize = 43;
const SYSCALL_CHDIR:    usize = 49;
const SYSCALL_OPENAT:   usize = 56;
const SYSCALL_CLOSE:    usize = 57;
const SYSCALL_PIPE:     usize = 59;
const SYSCALL_GETDENTS64: usize = 61;
const SYSCALL_LSEEK:    usize = 62;
const SYSCALL_READ:     usize = 63;
const SYSCALL_WRITE:    usize = 64;
const SYSCALL_READV:    usize = 65;
const SYSCALL_WRITEV:   usize = 66;
const SYSCALL_PREAD64:  usize = 67;
const SYSCALL_FSTATAT:  usize = 79;
const SYSCALL_FSTAT:    usize = 80;
const SYSCALL_UTIMENSAT:usize = 88;
const SYSCALL_EXIT:     usize = 93;
const SYSCALL_EXIT_GROUP:     usize = 94;
const SYSCALL_SET_TID_ADDRESS:     usize = 96;
const SYSCALL_FUTEX:    usize = 98;
const SYSCALL_NANOSLEEP:usize = 101;
const SYSCALL_CLOCK_GETTIME:usize = 113;
const SYSCALL_YIELD:    usize = 124;
const SYSCALL_KILL:     usize = 129;
const SYSCALL_RT_SIGACTION: usize = 134;
const SYSCALL_RT_SIGPROCMASK: usize = 135;
const SYSCALL_RT_SIGTIMEDWAIT: usize = 137;
const SYSCALL_RT_SIGRETURN: usize = 139;
const SYSCALL_TIMES:    usize = 153;
const SYSCALL_UNAME:    usize = 160;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID:   usize = 172;
const SYSCALL_GETPPID:  usize = 173;
const SYSCALL_GETEUID:  usize = 175;
const SYSCALL_GETEGID:  usize = 177;
const SYSCALL_GETTID:   usize = 178;
const SYSCALL_SOCKET:   usize = 198;
const SYSCALL_BIND:     usize = 200;
const SYSCALL_LISTEN:   usize = 201;
const SYSCALL_ACCEPT:   usize = 202;
const SYSCALL_CONNECT:  usize = 203;
const SYSCALL_GETSOCKNAME: usize = 204;
const SYSCALL_SENDTO:   usize = 206;
const SYSCALL_RECVFROM: usize = 207;
const SYSCALL_SETSOCKOPT: usize = 208;
const SYSCALL_BRK:      usize = 214;
const SYSCALL_MUNMAP:   usize = 215;
const SYSCALL_FORK:     usize = 220;
const SYSCALL_EXEC:     usize = 221;
const SYSCALL_MMAP:     usize = 222;
const SYSCALL_WAITPID:  usize = 260;
const SYSCALL_PRLIMIT64:usize = 261;

mod fs;         // 文件读写模块
mod process;    // 进程控制模块
mod thread;
mod sigset;
mod socket;

use fs::*;
use process::*;
use thread::*;
use sigset::*;
use socket::*;


/// 系统调用分发函数
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYSCALL_GETCWD =>   sys_getcwd(args[0] as *mut u8, args[1]),
        SYSCALL_DUP =>      sys_dup(args[0]),
        SYSCALL_DUP3 =>     sys_dup3(args[0], args[1]),
        SYSCALL_FCNTL =>    sys_fcntl(args[0] as isize,args[1], Option::<usize>::from(args[2])),
        SYSCALL_IOCTL=>     sys_ioctl(args[0],args[1],args[2] as *mut u8),
        SYSCALL_MKDIRAT =>  sys_mkdirat(args[0] as isize, args[1] as *const u8, args[2] as u32),
        SYSCALL_UNLINKAT=>  sys_unlinkat(args[0] as isize, args[1] as *const u8, args[2] as u32),
        SYSCALL_UMOUNT2=>   sys_umount(args[0] as *const u8, args[1]),
        SYSCALL_MOUNT=>     sys_mount(args[0] as *const u8, args[1] as *const u8, args[2] as *const u8, args[3], args[4] as *const u8),
        SYSCALL_STATFS=>    sys_statfs(args[0] as *const u8,args[1] as *const u8),
        SYSCALL_CHDIR=>     sys_chdir(args[0] as *const u8),
        SYSCALL_OPENAT =>   sys_openat(args[0] as isize, args[1] as *const u8, args[2] as u32, args[3] as u32),
        SYSCALL_CLOSE =>    sys_close(args[0]),
        SYSCALL_PIPE =>     sys_pipe(args[0] as *mut u32,args[1]),
        SYSCALL_GETDENTS64 => sys_getdents64(args[0] as isize, args[1] as *mut u8, args[2]),
        SYSCALL_LSEEK=>     sys_lseek(args[0],args[1],args[2]),
        SYSCALL_READ =>     sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE =>    sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_READV =>     sys_readv(args[0], args[1] as *const usize, args[2]),
        SYSCALL_WRITEV =>   sys_writev(args[0], args[1] as *const usize, args[2]),
        SYSCALL_FSTATAT=>     sys_fstatat(args[0] as isize, args[1] as *const u8,args[2] as *const usize,args[3]),

        SYSCALL_FSTAT=>     sys_fstat(args[0] as isize, args[1] as *mut u8),
        SYSCALL_UTIMENSAT=> sys_utimensat(args[0] as isize, args[1] as *const u8,args[2] as *const usize,args[3]),
        SYSCALL_EXIT =>     sys_exit(args[0] as i32),
        SYSCALL_EXIT_GROUP =>     sys_exit_group(args[0] as i32),
        SYSCALL_SET_TID_ADDRESS => sys_set_tid_address(args[0] as *mut usize),
        SYSCALL_FUTEX => sys_futex(),
        SYSCALL_NANOSLEEP=> sys_nanosleep(args[0] as *const u8),
        SYSCALL_CLOCK_GETTIME=> sys_clock_gettime(args[0],args[1] as *mut usize),
        SYSCALL_YIELD =>    sys_yield(),
        SYSCALL_KILL =>     sys_kill(args[0], args[1] as u32),
        SYSCALL_RT_SIGACTION => sys_rt_sigaction(),
        SYSCALL_RT_SIGPROCMASK =>     sys_rt_sigprocmask(args[0] as i32,args[1] as *const usize,args[2] as *const usize,args[3]),
        SYSCALL_RT_SIGTIMEDWAIT => sys_rt_sigtimedwait(),
        SYSCALL_RT_SIGRETURN =>     sys_rt_sigreturn(args[0] as *mut usize),
        SYSCALL_TIMES =>    sys_times(args[0] as *const u8),
        SYSCALL_UNAME =>    sys_uname(args[0] as *const u8),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *const u8),
        SYSCALL_GETPID =>   sys_getpid(),
        SYSCALL_GETPPID =>  sys_getppid(),
        SYSCALL_GETEUID =>  sys_geteuid(),
        SYSCALL_GETEGID =>  sys_getegid(),
        SYSCALL_FORK =>     sys_fork(args[0], args[1], args[2], args[3], args[4]),
        SYSCALL_EXEC =>     sys_exec(args[0] as *const u8, args[1] as *const usize,args[2] as *const usize),
        SYSCALL_GETTID =>   sys_gettid(),
        SYSCALL_SOCKET =>   sys_socket(),
        SYSCALL_BIND   =>   sys_bind(),
        SYSCALL_LISTEN =>   sys_listen(),
        SYSCALL_ACCEPT =>   sys_accept(),
        SYSCALL_CONNECT=>   sys_connect(),
        SYSCALL_GETSOCKNAME=> sys_getsockname(),
        SYSCALL_SENDTO =>   sys_sendto(),
        SYSCALL_RECVFROM=>  sys_recvfrom(args[0] as isize,args[1],args[2],args[3],args[4],args[5]),
        SYSCALL_SETSOCKOPT => sys_setsockopt(),
        SYSCALL_BRK =>      sys_brk(args[0]),
        SYSCALL_MMAP=>      sys_mmap(args[0], args[1], args[2], args[3], args[4] as isize, args[5]),
        SYSCALL_MUNMAP =>   sys_munmap(args[0], args[1]),
        SYSCALL_WAITPID =>  sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_PRLIMIT64=> sys_prlimit64(args[0], args[1], args[2] as *const u8, args[3] as *const u8),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

