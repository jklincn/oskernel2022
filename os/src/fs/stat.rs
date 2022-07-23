pub const S_IFDIR: u32 = 0o0040000;
pub const S_IFCHR: u32 = 0o0020000;
pub const S_IFBLK: u32 = 0o0060000;
pub const S_IFREG: u32 = 0o0100000;
pub const S_IFIFO: u32 = 0o0010000;
pub const S_IFLNK: u32 = 0o0120000;
pub const S_IFSOCK: u32 = 0o0140000;

#[derive(Debug)]
#[repr(C)]
pub struct Kstat {
    st_dev: u64,   // 包含文件的设备 ID
    st_ino: u64,   // 索引节点号
    st_mode: u32,  // 文件类型和模式
    st_nlink: u32, // 硬链接数
    st_uid: u32,   // 所有者的用户 ID
    st_gid: u32,   // 所有者的组 ID
    st_rdev: u64,  // 设备 ID（如果是特殊文件）
    __pad: u64,
    st_size: i64,    // 总大小，以字节为单位
    st_blksize: i32, // 文件系统 I/O 的块大小
    __pad2: i32,
    st_blocks: u64,     // 分配的 512B 块数
    st_atime_sec: i64,  // 上次访问时间
    st_atime_nsec: i64, // 上次访问时间（纳秒精度）
    st_mtime_sec: i64,  // 上次修改时间
    st_mtime_nsec: i64, // 上次修改时间（纳秒精度）
    st_ctime_sec: i64,  // 上次状态变化的时间
    st_ctime_nsec: i64, // 上次状态变化的时间（纳秒精度）
    __unused: [u32; 2],
}

impl Kstat {
    pub fn new() -> Self {
        Self {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: 0,
            st_blksize: 0,
            __pad2: 0,
            st_blocks: 0,
            st_atime_sec: 0,
            st_atime_nsec: 0,
            st_mtime_sec: 0,
            st_mtime_nsec: 0,
            st_ctime_sec: 0,
            st_ctime_nsec: 0,
            __unused: [0; 2],
        }
    }

    pub fn init(&mut self, st_size: i64, st_blksize: i32, st_blocks: u64, st_mode: u32, time: u64) {
        self.st_nlink = 1;
        self.st_size = st_size;
        self.st_blksize = st_blksize;
        self.st_blocks = st_blocks;
        self.st_mode = st_mode;
        // 参见 simple-fat32 中注释
        if time == 12345 {
            self.st_atime_sec = 1 << 32;
            self.st_mtime_sec = 1 << 32;
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

pub struct Timespec {
    pub tv_sec: u64,
    pub tv_nsec: u64,
}

#[repr(C)]
pub struct Statfs {
    f_type: u64,
    f_bsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_fsid: u64,
    f_namelen: u64,
    f_frsize: u64,
    f_flag: u64,
    f_spare: [u64; 4],
}

impl Statfs {
    pub fn new() -> Self {
        Self {
            f_type:1,
            f_bsize: 512,
            f_blocks: 12345,
            f_bfree: 1234,
            f_bavail: 123,
            f_files: 1000,
            f_ffree: 100,
            f_fsid: 1,
            f_namelen: 123,
            f_frsize: 4096,
            f_flag: 123,
            f_spare: [0; 4],
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}
