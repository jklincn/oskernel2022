use alloc::collections::BTreeMap;
use lazy_static::*;

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
const SYSCALL_FACCESSAT:usize = 48;
const SYSCALL_CHDIR:    usize = 49;
const SYSCALL_OPENAT:   usize = 56;
const SYSCALL_CLOSE:    usize = 57;
const SYSCALL_PIPE:     usize = 59;
const SYSCALL_GETDENTS64:usize = 61;
const SYSCALL_LSEEK:    usize = 62;
const SYSCALL_READ:     usize = 63;
const SYSCALL_WRITE:    usize = 64;
const SYSCALL_READV:    usize = 65;
const SYSCALL_WRITEV:   usize = 66;
const SYSCALL_PREAD64:  usize = 67;
const SYSCALL_SENDFILE: usize = 71;
const SYSCALL_PSELECT6: usize = 72;
const SYSCALL_PPOLL:    usize = 73;
const SYSCALL_READLINKAT:usize = 78;
const SYSCALL_NEWFSTATAT:  usize = 79;
const SYSCALL_FSTAT:    usize = 80;
const SYSCALL_FSYNC:    usize = 82;
const SYSCALL_UTIMENSAT:usize = 88;
const SYSCALL_EXIT:     usize = 93;
const SYSCALL_EXIT_GROUP:     usize = 94;
const SYSCALL_SET_TID_ADDRESS:usize = 96;
const SYSCALL_FUTEX:    usize = 98;
const SYSCALL_NANOSLEEP:usize = 101;
const SYSCALL_SETITIMER:usize = 103;
const SYSCALL_CLOCK_GETTIME:usize = 113;
const SYSCALL_SYSLOG:   usize = 116;
const SYSCALL_YIELD:    usize = 124;
const SYSCALL_KILL:     usize = 129;
const SYSCALL_TGKILL:    usize = 131;
const SYSCALL_RT_SIGACTION: usize = 134;
const SYSCALL_RT_SIGPROCMASK: usize = 135;
const SYSCALL_RT_SIGTIMEDWAIT: usize = 137;
const SYSCALL_RT_SIGRETURN: usize = 139;
const SYSCALL_TIMES:    usize = 153;
const SYSCALL_SETPGID:  usize = 154;
const SYSCALL_GETPGID:  usize = 155;
const SYSCALL_UNAME:    usize = 160;
const SYSCALL_GETRUSAGE:usize = 165;
const SYSCALL_UMASK:    usize = 166;
const SYSCALL_GETTIMEOFDAY: usize = 169;
const SYSCALL_GETPID:   usize = 172;
const SYSCALL_GETPPID:  usize = 173;
const SYSCALL_GETUID:   usize = 174;
const SYSCALL_GETEUID:  usize = 175;
const SYSCALL_GETEGID:  usize = 177;
const SYSCALL_GETTID:   usize = 178;
const SYSCALL_SYSINFO:  usize = 179;
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
const SYSCALL_MPROTECT: usize = 226;
const SYSCALL_MSYNC:    usize = 227;
const SYSCALL_MADVISE:  usize = 233;
const SYSCALL_WAITPID:  usize = 260;
const SYSCALL_PRLIMIT64:usize = 261;
const SYSCALL_RENAMEAT2: usize = 276;

mod fs;
mod process;
mod sigset;
mod socket;
mod errno;

use fs::*;
use process::*;
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
        SYSCALL_FACCESSAT=> sys_faccessat(),
        SYSCALL_CHDIR=>     sys_chdir(args[0] as *const u8),
        SYSCALL_OPENAT =>   sys_openat(args[0] as isize, args[1] as *const u8, args[2] as u32, args[3] as u32),
        SYSCALL_CLOSE =>    sys_close(args[0]),
        SYSCALL_PIPE =>     sys_pipe(args[0] as *mut u32,args[1]),
        SYSCALL_GETDENTS64=>sys_getdents64(args[0] as isize, args[1] as *mut u8, args[2]),
        SYSCALL_LSEEK=>     sys_lseek(args[0],args[1],args[2]),
        SYSCALL_READ =>     sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE =>    sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_READV =>    sys_readv(args[0], args[1] as *const usize, args[2]),
        SYSCALL_WRITEV =>   sys_writev(args[0], args[1] as *const usize, args[2]),
        SYSCALL_PREAD64=>   sys_pread64(args[0], args[1] as *const u8, args[2],args[3]),
        SYSCALL_SENDFILE=>  sys_sendfile(args[0], args[1], args[2],args[3]),
        SYSCALL_PSELECT6=>  sys_pselect(args[0] as usize, args[1] as *mut u8, args[2] as *mut u8, args[3] as *mut u8, args[4] as *mut usize),
        SYSCALL_PPOLL  =>   sys_ppoll(),
        SYSCALL_READLINKAT =>sys_readlinkat(args[0] as isize,args[1] as *const u8 ,args[2] as *const u8,args[3]),
        SYSCALL_NEWFSTATAT=>sys_newfstatat(args[0] as isize, args[1] as *const u8,args[2] as *const usize,args[3]),
        SYSCALL_FSTAT=>     sys_fstat(args[0] as isize, args[1] as *mut u8),
        SYSCALL_FSYNC=>     0,
        SYSCALL_UTIMENSAT=> sys_utimensat(args[0] as isize, args[1] as *const u8,args[2] as *const usize,args[3]),
        SYSCALL_EXIT =>     sys_exit(args[0] as i32),
        SYSCALL_EXIT_GROUP=>sys_exit_group(args[0] as i32),
        SYSCALL_SET_TID_ADDRESS=>sys_set_tid_address(args[0] as *mut usize),
        SYSCALL_FUTEX =>    sys_futex(),
        SYSCALL_NANOSLEEP=> sys_nanosleep(args[0] as *const u8),
        SYSCALL_SETITIMER=> 0,
        SYSCALL_CLOCK_GETTIME=> sys_clock_gettime(args[0],args[1] as *mut u64),
        SYSCALL_SYSLOG =>   0,
        SYSCALL_YIELD =>    sys_yield(),
        SYSCALL_KILL =>     sys_kill(args[0], args[1] as u32),
        SYSCALL_TGKILL=>    0,
        SYSCALL_RT_SIGACTION => sys_rt_sigaction(),
        SYSCALL_RT_SIGPROCMASK=>sys_rt_sigprocmask(args[0] as i32,args[1] as *const usize,args[2] as *const usize,args[3]),
        SYSCALL_RT_SIGTIMEDWAIT=>sys_rt_sigtimedwait(),
        SYSCALL_RT_SIGRETURN => sys_rt_sigreturn(args[0] as *mut usize),
        SYSCALL_TIMES =>    sys_times(args[0] as *const u8),
        SYSCALL_SETPGID=>   sys_setpgid(),
        SYSCALL_GETPGID =>  sys_getpgid(),
        SYSCALL_UNAME =>    sys_uname(args[0] as *const u8),
        SYSCALL_GETRUSAGE=> sys_getrusage(args[0] as isize, args[1] as *mut u8),
        SYSCALL_UMASK =>    sys_umask(),
        SYSCALL_GETTIMEOFDAY => sys_gettimeofday(args[0] as *const u8),
        SYSCALL_GETPID =>   sys_getpid(),
        SYSCALL_GETPPID =>  sys_getppid(),
        SYSCALL_GETUID =>   sys_getuid(),
        SYSCALL_GETEUID =>  sys_geteuid(),
        SYSCALL_GETEGID =>  sys_getegid(),
        SYSCALL_FORK =>     sys_fork(args[0], args[1], args[2], args[3], args[4]),
        SYSCALL_EXEC =>     sys_exec(args[0] as *const u8, args[1] as *const usize,args[2] as *const usize),
        SYSCALL_GETTID =>   sys_gettid(),
        SYSCALL_SYSINFO=>   sys_sysinfo(),
        SYSCALL_SOCKET =>   sys_socket(),
        SYSCALL_BIND   =>   sys_bind(),
        SYSCALL_LISTEN =>   sys_listen(),
        SYSCALL_ACCEPT =>   sys_accept(),
        SYSCALL_CONNECT=>   sys_connect(),
        SYSCALL_GETSOCKNAME=>sys_getsockname(),
        SYSCALL_SENDTO =>   sys_sendto(),
        SYSCALL_RECVFROM=>  sys_recvfrom(args[0] as isize,args[1],args[2],args[3],args[4],args[5]),
        SYSCALL_SETSOCKOPT =>sys_setsockopt(),
        SYSCALL_BRK =>      sys_brk(args[0]),
        SYSCALL_MMAP=>      sys_mmap(args[0], args[1], args[2], args[3], args[4] as isize, args[5]),
        SYSCALL_MUNMAP =>   sys_munmap(args[0], args[1]),
        SYSCALL_MPROTECT=>  0,
        SYSCALL_MSYNC=>     0,
        SYSCALL_MADVISE=>   sys_madvise(args[0] as *const u8, args[1], args[2]),
        SYSCALL_WAITPID =>  sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_PRLIMIT64=> sys_prlimit64(args[0], args[1], args[2] as *const u8, args[3] as *const u8),
        SYSCALL_RENAMEAT2=> sys_renameat2(args[0] as isize, args[1] as *const u8,args[2] as isize, args[3] as *const u8, args[4] as u32
        ),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

lazy_static! {
    pub static ref SYSCALL_NAME: BTreeMap<usize,&'static str> = {
        let mut tmp = BTreeMap::new();
        tmp.insert(SYSCALL_GETCWD, "getcwd");
        tmp.insert(SYSCALL_DUP, "dup");
        tmp.insert(SYSCALL_DUP3, "dup3");
        tmp.insert(SYSCALL_FCNTL, "fcntl");
        tmp.insert(SYSCALL_IOCTL, "ioctl");
        tmp.insert(SYSCALL_MKDIRAT, "mkdirat");
        tmp.insert(SYSCALL_UNLINKAT, "unlinkat");
        tmp.insert(SYSCALL_UMOUNT2, "umount2");
        tmp.insert(SYSCALL_MOUNT, "mount");
        tmp.insert(SYSCALL_STATFS, "statfs");
        tmp.insert(SYSCALL_FACCESSAT, "faccessat");
        tmp.insert(SYSCALL_CHDIR, "chdir");
        tmp.insert(SYSCALL_OPENAT, "openat");
        tmp.insert(SYSCALL_CLOSE, "close");
        tmp.insert(SYSCALL_PIPE, "pipe");
        tmp.insert(SYSCALL_GETDENTS64, "getdents64");
        tmp.insert(SYSCALL_LSEEK, "lseek");
        tmp.insert(SYSCALL_READ, "read");
        tmp.insert(SYSCALL_WRITE, "write");
        tmp.insert(SYSCALL_READV, "readv");
        tmp.insert(SYSCALL_WRITEV, "writev");
        tmp.insert(SYSCALL_PREAD64, "pread64");
        tmp.insert(SYSCALL_SENDFILE, "sendfile");
        tmp.insert(SYSCALL_PSELECT6, "pselect6");
        tmp.insert(SYSCALL_PPOLL, "ppoll");
        tmp.insert(SYSCALL_READLINKAT, "readlinkat");
        tmp.insert(SYSCALL_NEWFSTATAT, "newfstatat");
        tmp.insert(SYSCALL_FSTAT, "fstat");
        tmp.insert(SYSCALL_FSYNC, "fsync");
        tmp.insert(SYSCALL_UTIMENSAT, "utimensat");
        tmp.insert(SYSCALL_EXIT, "exit");
        tmp.insert(SYSCALL_EXIT_GROUP, "exit_group");
        tmp.insert(SYSCALL_SET_TID_ADDRESS, "set_tid_address");
        tmp.insert(SYSCALL_FUTEX, "futex");
        tmp.insert(SYSCALL_NANOSLEEP, "nanosleep");
        tmp.insert(SYSCALL_SETITIMER, "setitimer");
        tmp.insert(SYSCALL_CLOCK_GETTIME, "clock_gettime");
        tmp.insert(SYSCALL_SYSLOG, "syslog");
        tmp.insert(SYSCALL_YIELD, "yield");
        tmp.insert(SYSCALL_KILL, "kill");
        tmp.insert(SYSCALL_TGKILL, "tgkill");
        tmp.insert(SYSCALL_RT_SIGACTION, "rt_sigaction");
        tmp.insert(SYSCALL_RT_SIGPROCMASK, "rt_sigprocmask");
        tmp.insert(SYSCALL_RT_SIGTIMEDWAIT, "rt_sigtimedwait");
        tmp.insert(SYSCALL_RT_SIGRETURN, "rt_sigreturn");
        tmp.insert(SYSCALL_TIMES, "times");
        tmp.insert(SYSCALL_SETPGID, "setpgid");
        tmp.insert(SYSCALL_GETPGID, "getpgid");
        tmp.insert(SYSCALL_UNAME, "uname");
        tmp.insert(SYSCALL_GETRUSAGE, "getrusage");
        tmp.insert(SYSCALL_UMASK, "umask");
        tmp.insert(SYSCALL_GETTIMEOFDAY, "gettimeofday");
        tmp.insert(SYSCALL_GETPID, "getpid");
        tmp.insert(SYSCALL_GETPPID, "getppid");
        tmp.insert(SYSCALL_GETUID, "getuid");
        tmp.insert(SYSCALL_GETEUID, "geteuid");
        tmp.insert(SYSCALL_GETEGID, "getegid");
        tmp.insert(SYSCALL_GETTID, "gettid");
        tmp.insert(SYSCALL_SYSINFO, "sysinfo");
        tmp.insert(SYSCALL_SOCKET, "socket");
        tmp.insert(SYSCALL_BIND, "bind");
        tmp.insert(SYSCALL_LISTEN, "listen");
        tmp.insert(SYSCALL_ACCEPT, "accept");
        tmp.insert(SYSCALL_CONNECT, "connect");
        tmp.insert(SYSCALL_GETSOCKNAME, "getsockname");
        tmp.insert(SYSCALL_SENDTO, "sendto");
        tmp.insert(SYSCALL_RECVFROM, "recvfrom");
        tmp.insert(SYSCALL_SETSOCKOPT, "setsockopt");
        tmp.insert(SYSCALL_BRK, "brk");
        tmp.insert(SYSCALL_MUNMAP, "mummap");
        tmp.insert(SYSCALL_FORK, "fork");
        tmp.insert(SYSCALL_EXEC, "exec");
        tmp.insert(SYSCALL_MMAP, "mmap");
        tmp.insert(SYSCALL_MPROTECT, "mprotect");
        tmp.insert(SYSCALL_MSYNC, "msync");
        tmp.insert(SYSCALL_MADVISE, "madvise");
        tmp.insert(SYSCALL_WAITPID, "waitpid");
        tmp.insert(SYSCALL_PRLIMIT64, "prlimit64");
        tmp.insert(SYSCALL_RENAMEAT2, "renameat2");
        tmp
    };
}
