pub const RESOURCE_KIND_NUMBER: usize = 17;

// pub const RLIMIT_CPU: usize = 0;
// pub const RLIMIT_FSIZE: usize = 1;
// pub const RLIMIT_DATA: usize = 2;
// pub const RLIMIT_STACK: usize = 3;
// pub const RLIMIT_CORE: usize = 4;

// pub const RLIMIT_RSS: usize = 5;
// pub const RLIMIT_NPROC: usize = 6;
pub const RLIMIT_NOFILE: usize = 7;
// pub const RLIMIT_MEMLOCK: usize = 8;
// pub const RLIMIT_AS: usize = 9;

// pub const RLIMIT_LOCKS: usize = 10;
// pub const RLIMIT_SIGPENDING: usize = 11;
// pub const RLIMIT_MSGQUEUE: usize = 12;
// pub const RLIMIT_NICE: usize = 13;
// pub const RLIMIT_RTPRIO: usize = 14;
// pub const RLIMIT_RTTIME: usize = 15;
// pub const RLIMIT_NLIMITS: usize = 16;

#[derive(Clone, Copy)]
pub struct RLimit {
    pub rlim_cur: usize,
    pub rlim_max: usize,
}

impl RLimit {
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}
