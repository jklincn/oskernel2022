/// # 系统信息模块
/// `os/src/task/info.rs`
/// ```
/// pub struct Utsname
/// pub struct CloneFlags
/// ```
//
//use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

pub struct Utsname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

lazy_static! {
    pub static ref UTSNAME: Mutex<Utsname> = Mutex::new(Utsname::new());
}

impl Utsname {
    pub fn new() -> Self {
        Self {
            sysname: Utsname::str2u8("Linux"),
            nodename: Utsname::str2u8("untuntu"),
            release: Utsname::str2u8("5.0"),
            version: Utsname::str2u8("5.13"),
            machine: Utsname::str2u8("riscv64"),
            domainname: Utsname::str2u8("Jeremy_test"),
        }
    }

    pub fn str2u8(str: &str) -> [u8; 65] {
        let mut arr: [u8; 65] = [0; 65];
        let cstr = str.as_bytes();
        let len = str.len();
        for i in 0..len {
            arr[i] = cstr[i];
        }
        arr
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

bitflags! {
    pub struct CloneFlags: usize{
        const SIGCHLD = 17;
        const CSIGNAL	    =	0x000000ff;	/* signal mask to be sent at exit */
        const CLONE_VM	    =   0x00000100;/* set if VM shared between processes */
        const CLONE_FS      =	0x00000200;	/* set if fs info shared between processes */
        const CLONE_FILES   =	0x00000400;/* set if open files shared between processes */
        const CLONE_SIGHAND =	0x00000800;	/* set if signal handlers and blocked signals shared */
        const CLONE_PIDFD	=   0x00001000;	/* set if a pidfd should be placed in parent */
        const CLONE_PTRACE	=   0x00002000;/* set if we want to let tracing continue on the child too */
        const CLONE_VFORK	=   0x00004000;/* set if the parent wants the child to wake it up on mm_release */
        const CLONE_PARENT	=   0x00008000;/* set if we want to have the same parent as the cloner */
        const CLONE_THREAD	=   0x00010000;/* Same thread group? */
        const CLONE_NEWNS	=   0x00020000;/* New mount namespace group */
        const CLONE_SYSVSEM =	0x00040000;/* share system V SEM_UNDO semantics */
        const CLONE_SETTLS	=   0x00080000;	/* create a new TLS for the child */
        const CLONE_PARENT_SETTID	=   0x00100000;/* set the TID in the parent */
        const CLONE_CHILD_CLEARTID	=   0x00200000;/* clear the TID in the child */
        const CLONE_DETACHED		=   0x00400000;/* Unused, ignored */
        const CLONE_UNTRACED	    =	0x00800000;	/* set if the tracing process can't force CLONE_PTRACE on this clone */
        const CLONE_CHILD_SETTID	=   0x01000000;/* set the TID in the child */
        const CLONE_NEWCGROUP	    =	0x02000000;	/* New cgroup namespace */
        const CLONE_NEWUTS	=	0x04000000;	/* New utsname namespace */
        const CLONE_NEWIPC	=	0x08000000;	/* New ipc namespace */
        const CLONE_NEWUSER	=	0x10000000;	/* New user namespace */
        const CLONE_NEWPID	=	0x20000000;	/* New pid namespace */
        const CLONE_NEWNET	=	0x40000000;	/* New network namespace */
        const CLONE_IO		=   0x80000000;/* Clone io context */

    }
}
