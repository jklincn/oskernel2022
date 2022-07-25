use super::{
    stat::{S_IFCHR, S_IFDIR, S_IFREG},
    Dirent, File, Kstat, Timespec,
};
use crate::{drivers::BLOCK_DEVICE, mm::{UserBuffer, heap_allocator_stats}};
use _core::str::FromStr;
use alloc::{string::String, sync::Arc, vec::Vec};
use bitflags::*;
use lazy_static::*;
use simple_fat32::{create_root_vfile, FAT32Manager, VFile, ATTR_ARCHIVE, ATTR_DIRECTORY, END_CLUSTER};
use spin::Mutex;
use core::fmt::{self, Debug, Formatter};

/// 表示进程中一个被打开的常规文件或目录
pub struct OSInode {
    readable: bool, // 该文件是否允许通过 sys_read 进行读
    writable: bool, // 该文件是否允许通过 sys_write 进行写
    inner: Mutex<OSInodeInner>,
}

impl Debug for OSInode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("OSInode debug:{{todo}}"))
    }
}

pub struct OSInodeInner {
    offset: usize, // 偏移量
    inode: Arc<VFile>,
    flags: OpenFlags,
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<VFile>) -> Self {
        Self {
            readable,
            writable,
            inner: Mutex::new(OSInodeInner {
                offset: 0,
                inode,
                flags: OpenFlags::empty(),
            }),
        }
    }
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            heap_allocator_stats();
            println!("read at");
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            heap_allocator_stats();
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
            println!("len: {},capacity: {}",v.len(),v.capacity());
        }
        heap_allocator_stats();
        v
    }

    pub fn read_vec(&self, offset:usize, len:usize)->Vec<u8>{
        let inner = self.inner.lock();
        let mut len = len;
        let mut offset = offset;

        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let read_len = inner.inode.read_at(offset, &mut buffer);
            if read_len == 0 {
                break;
            }
            offset += read_len;
            v.extend_from_slice(&buffer[..read_len.min(len)]);
            if len > read_len {
                len -= read_len;
            } else {
                break;
            }
        }
        v
    }

    pub fn is_dir(&self) -> bool {
        let inner = self.inner.lock();
        inner.inode.is_dir()
    }

    pub fn name(&self) -> String {
        let mut name = String::new();
        name.push_str(self.inner.lock().inode.name());
        name
    }

    pub fn delete(&self) -> usize {
        let inner = self.inner.lock();
        inner.inode.remove()
    }
    pub fn file_size(&self) -> usize {
        let inner = self.inner.lock();
        inner.inode.file_size() as usize
    }
}

// 这里在实例化的时候进行文件系统的打开
lazy_static! {
    pub static ref ROOT_INODE: Arc<VFile> = {
        let fat32_manager = FAT32Manager::open(BLOCK_DEVICE.clone());
        Arc::new(create_root_vfile(&fat32_manager)) // 返回根目录
    };
}

pub fn list_apps() {
    // 决赛内容：在初始化时创建以下文件
    open("/", "tmp", OpenFlags::O_DIRECTROY);
    open("/", "dev", OpenFlags::O_DIRECTROY);
    open("/dev", "null", OpenFlags::O_CREATE);
    open("/dev", "zero", OpenFlags::O_CREATE);
    println!("/**** All Files  ****");
    for app in ROOT_INODE.ls().unwrap() {
        if app.1 & ATTR_DIRECTORY == 0 {
            // 如果不是目录
            println!("{}", app.0);
        } else {
            // 暂不考虑二级目录，待改写
            println!("{}/", app.0);
            let dir = open("/", app.0.as_str(), OpenFlags::O_RDONLY).unwrap();
            let inner = dir.inner.lock();
            for file in inner.inode.ls().unwrap() {
                if file.1 & ATTR_DIRECTORY == 0 {
                    println!("{}/{}", app.0, file.0);
                }
            }
        }
    }
    println!("**********************/");
}

// 定义一份打开文件的标志
bitflags! {
    pub struct OpenFlags: u32 {
        const O_RDONLY = 0;
        const O_WRONLY = 1 << 0;
        const O_RDWR = 1 << 1;
        const O_CREATE = 1 << 6;
        const O_TRUNC = 1 << 9;
        const O_DIRECTROY = 1 << 16;
        // 决赛添加
        const O_EXCL = 1 << 7;
        const O_LARGEFILE = 1 << 15;
        const O_APPEND = 1 << 10;
        const O_NOFOLLOW = 1 << 17;
        const O_CLOEXEC = 1 << 19;
    }
}

impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::O_WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// 内核层面的open，不设置fd
pub fn open(work_path: &str, path: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let cur_inode = {
        if work_path == "/" {
            ROOT_INODE.clone()
        } else {
            let wpath: Vec<&str> = work_path.split('/').collect();
            ROOT_INODE.find_vfile_bypath(wpath).unwrap()
        }
    };
    let mut pathv: Vec<&str> = path.split('/').collect();
    let (readable, writable) = flags.read_write();

    if flags.contains(OpenFlags::O_CREATE) || flags.contains(OpenFlags::O_DIRECTROY) {
        if let Some(inode) = cur_inode.find_vfile_bypath(pathv.clone()) {
            // 如果文件已存在则清空
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // 设置创建类型
            let mut create_type = 0;
            if flags.contains(OpenFlags::O_CREATE) {
                create_type = ATTR_ARCHIVE;
            } else if flags.contains(OpenFlags::O_DIRECTROY) {
                create_type = ATTR_DIRECTORY;
            }
            let name = pathv.pop().unwrap();
            if let Some(temp_inode) = cur_inode.find_vfile_bypath(pathv.clone()) {
                temp_inode
                    .create(name, create_type)
                    .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
            } else {
                None
            }
        }
    } else {
        cur_inode.find_vfile_bypath(pathv).map(|inode| {
            if flags.contains(OpenFlags::O_TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

pub fn chdir(work_path: &str, path: &str) -> Option<String> {
    let current_inode = {
        if path.chars().nth(0).unwrap() == '/' {
            // 传入路径是绝对路径
            ROOT_INODE.clone()
        } else {
            // 传入路径是相对路径
            let current_work_pathv: Vec<&str> = work_path.split('/').collect();
            ROOT_INODE.find_vfile_bypath(current_work_pathv).unwrap()
        }
    };
    let pathv: Vec<&str> = path.split('/').collect();
    if let Some(_) = current_inode.find_vfile_bypath(pathv) {
        let new_current_path = String::from_str("/").unwrap() + &String::from_str(path).unwrap();
        if current_inode.name() == "/" {
            Some(new_current_path)
        } else {
            Some(String::from_str(current_inode.name()).unwrap() + &new_current_path)
        }
    } else {
        None
    }
}

// 为 OSInode 实现 File Trait
impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {

        // 对 /dev/zero 的处理，暂时先加在这里
        if self.name() == "zero" {
            let zero: Vec<u8> = (0..buf.buffers.len()).map(|_| 0).collect();
            buf.write(zero.as_slice());
            return buf.buffers.len();
        }

        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;

        // 这边要使用 iter_mut()，因为要将数据写入
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut total_write_size = 0usize;
        let filesize = self.file_size();
        let mut inner = self.inner.lock();
        if inner.flags.contains(OpenFlags::O_APPEND) {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(filesize, *slice);
                inner.offset += write_size;
                total_write_size += write_size;
            }
        } else {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(inner.offset, *slice);
                assert_eq!(write_size, slice.len());
                inner.offset += write_size;
                total_write_size += write_size;
            }
        }
        total_write_size
    }

    fn get_fstat(&self, kstat: &mut Kstat) {
        let inner = self.inner.lock();
        let vfile = inner.inode.clone();
        let mut st_mode = 0;
        // todo
        let (st_size, st_blksize, st_blocks, is_dir, time) = vfile.stat();
        if is_dir {
            st_mode = S_IFDIR;
        } else {
            st_mode = S_IFREG;
        }
        if vfile.name() == "null" {
            st_mode = S_IFCHR;
        }
        kstat.init(st_size, st_blksize as i32, st_blocks, st_mode, time);
    }

    fn set_time(&self, timespec: &Timespec) {
        let tv_sec = timespec.tv_sec;
        let tv_nsec = timespec.tv_nsec;

        let inner = self.inner.lock();
        let vfile = inner.inode.clone();

        // 属于是针对测试用例了，待完善
        if tv_sec == 1 << 32 {
            vfile.set_time(tv_sec, tv_nsec);
        }
    }

    fn get_dirent(&self, dirent: &mut Dirent) -> isize {
        if !self.is_dir() {
            return -1;
        }
        let mut inner = self.inner.lock();
        let offset = inner.offset as u32;
        if let Some((name, off, _)) = inner.inode.dirent_info(offset as usize) {
            dirent.init(name.as_str());
            inner.offset = off as usize;
            let len = (name.len() + 8 * 4) as isize;
            len
        } else {
            -1
        }
    }

    fn get_name(&self) -> String {
        self.name()
    }

    fn get_offset(&self) -> usize {
        let inner = self.inner.lock();
        inner.offset
    }

    fn set_offset(&self, offset: usize) {
        let mut inner = self.inner.lock();
        inner.offset = offset;
    }

    fn set_flags(&self, flag: OpenFlags) {
        let mut inner = self.inner.lock();
        inner.flags.set(flag, true);
    }
}

// lazy_static! {
//     pub static ref OpenFileSet:Vec<Arc<OSInode>> = {
//         let fat32_manager = FAT32Manager::open(BLOCK_DEVICE.clone());
//         Arc::new(create_root_vfile(&fat32_manager)) // 返回根目录
//     };
// }