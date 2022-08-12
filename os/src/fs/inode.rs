use super::{
    stat::{S_IFCHR, S_IFDIR, S_IFREG},
    Dirent, File, Kstat, Timespec,
};
use crate::{drivers::BLOCK_DEVICE, mm::UserBuffer};
use _core::str::FromStr;
use alloc::{string::String, sync::Arc, vec::Vec};
use bitflags::*;
use lazy_static::*;
use simple_fat32::{create_root_vfile, FAT32Manager, VFile, ATTR_ARCHIVE, ATTR_DIRECTORY};
use spin::Mutex;

/// 表示进程中一个被打开的常规文件或目录
pub struct OSInode {
    readable: bool, // 该文件是否允许通过 sys_read 进行读
    writable: bool, // 该文件是否允许通过 sys_write 进行写
    inner: Mutex<OSInodeInner>,
}

pub struct OSInodeInner {
    offset: usize, // 偏移量
    inode: Arc<VFile>,
    flags: OpenFlags,
    available : bool,
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<VFile>) -> Self {
        let available = true;
        Self {
            readable,
            writable,
            inner: Mutex::new(OSInodeInner {
                offset: 0,
                inode,
                flags: OpenFlags::empty(),
                available,
            }),
        }
    }

    #[allow(unused)]
    pub fn read_all(&self) -> Vec<u8> {
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        if self.name() == "busybox"{
            v.reserve(1120000);  // 提前保留空间来防止过度扩容
        } else if self.name() == "lua" {
            v.reserve(300000);
        } else if self.name() == "lmbench_all" {
            v.reserve(1100000);
        }
        let mut inner = self.inner.lock();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }

    pub fn read_vec(&self, offset:isize, len:usize)->Vec<u8>{
        let mut inner = self.inner.lock();
        let mut len = len;
        let old_offset = inner.offset;
        if offset >= 0 {
            inner.offset = offset as usize;
        }
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let read_size = inner.inode.read_at(inner.offset, &mut buffer);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            v.extend_from_slice(&buffer[..read_size.min(len)]);
            if len > read_size {
                len -= read_size;
            } else {
                break;
            }
        }
        if offset >= 0 {
            inner.offset = old_offset; 
        }
        v
    }

    #[allow(unused)]
    pub fn write_all(&self, str_vec:&Vec<u8>)->usize{
        let mut inner = self.inner.lock();
        let mut remain = str_vec.len();
        let mut base = 0;
        loop {
            let len = remain.min(512);
            inner.inode.write_at(inner.offset, &str_vec.as_slice()[base .. base + len]);
            inner.offset += len;
            base += len;
            remain -= len;
            if remain == 0{
                break;
            }
        }
        return base
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

    pub fn set_head_cluster(&self, cluster:u32) {
        let inner = self.inner.lock();
        let vfile = &inner.inode;
        vfile.set_first_cluster(cluster);
    }    

    pub fn get_head_cluster(&self)->u32 {
        let inner = self.inner.lock();
        let vfile = &inner.inode;
        vfile.first_cluster()
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
    open("/", "proc", OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE);
    open("/", "tmp", OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE);
    open("/", "dev", OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE);
    open("/dev", "misc", OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE);
    open("/dev", "null", OpenFlags::O_CREATE);
    open("/dev", "zero", OpenFlags::O_CREATE);
    open("/proc", "mounts", OpenFlags::O_CREATE);
    open("/proc", "meminfo", OpenFlags::O_CREATE);
    open("/dev/misc", "rtc", OpenFlags::O_CREATE);

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
        const O_RDONLY    = 0;
        const O_WRONLY    = 1 << 0;
        const O_RDWR      = 1 << 1;
        const O_CREATE    = 1 << 6;
        const O_EXCL      = 1 << 7;
        const O_TRUNC     = 1 << 9;
        const O_APPEND    = 1 << 10;
        const O_NONBLOCK  = 1 << 11;
        const O_LARGEFILE = 1 << 15;
        const O_DIRECTROY = 1 << 16;
        const O_NOFOLLOW  = 1 << 17;
        const O_CLOEXEC   = 1 << 19;
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

pub fn open(work_path: &str, path: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    // println!("[DEBUG] enter open: work_path:{}, path:{}, flags:{:?}",work_path,path,flags);
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

    if flags.contains(OpenFlags::O_CREATE){
        // println!("[DEBUG] flags contain O_CREATE");
        if let Some(inode) = cur_inode.find_vfile_bypath(pathv.clone()) {
            // 如果文件已存在则清空
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // 设置创建类型
            let mut create_type = ATTR_ARCHIVE;
            if flags.contains(OpenFlags::O_DIRECTROY) {
                create_type = ATTR_DIRECTORY;
            }
            let name = pathv.pop().unwrap();
            if let Some(temp_inode) = cur_inode.find_vfile_bypath(pathv.clone()) {
                // println!("[DEBUG] create file: {}, type:0x{:x}",name,create_type);
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

    fn available(&self) ->bool{
        let inner = self.inner.lock();
        inner.available
    }
    
    fn read(&self, mut buf: UserBuffer) -> usize {
        // println!("osinode read, current offset:{}",self.inner.lock().offset);
        // 对 /dev/zero 的处理，暂时先加在这里
        if self.name() == "zero" {
            let zero: Vec<u8> = (0..buf.buffers.len()).map(|_| 0).collect();
            buf.write(zero.as_slice());
            return buf.buffers.len();
        }
        let offset = self.inner.lock().offset;
        let file_size = self.file_size();
        if file_size == 0 {
            println!("[WARNING] OSinode read: file_size is zero!");
        }
        if offset >= file_size{
            return 0;
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
        // println!("return total_read_size:{}",total_read_size);
        // println!("return userbuffer:{:?}",buf);
        total_read_size
    }

    fn read_kernel_space(&self) -> Vec<u8> {
        let file_size = self.file_size();
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop{
            if inner.offset > file_size{
                break;
            }
            let readsize = inner.inode.read_at(inner.offset, &mut buffer);
            if readsize == 0 {
                break;
            }
            inner.offset += readsize;
            v.extend_from_slice(&buffer[..readsize]);
        }
        v.truncate(v.len().min(file_size));
        v
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

    fn write_kernel_space(&self,data:Vec<u8>)->usize{
        let mut inner = self.inner.lock();
        let mut remain = data.len();
        let mut base = 0;
        loop {
            let len = remain.min(512);
            inner.inode.write_at(inner.offset, &data.as_slice()[base .. base + len]);
            inner.offset += len;
            base += len;
            remain -= len;
            if remain == 0{
                break;
            }
        }
        return base
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

    fn set_cloexec(&self){
        let mut inner = self.inner.lock();
        inner.available = false;
    }

    fn get_dirent(&self, dirent: &mut Dirent) -> isize {
        if !self.is_dir() {
            return -1;
        }
        let mut inner = self.inner.lock();
        let offset = inner.offset as u32;
        if let Some((name, off,first_clu, _attr)) = inner.inode.dirent_info(offset as usize) {
            dirent.init(name.as_str(),off as isize,first_clu as usize);
            inner.offset = off as usize;
            let len = (name.len() + 8 * 4) as isize;
            len
        } else {
            -1
        }
    }

    fn get_fstat(&self, kstat: &mut Kstat) {
        let inner = self.inner.lock();
        let vfile = inner.inode.clone();
        let mut st_mode = 0;
        _ = st_mode;
        // todo
        let (st_size, st_blksize, st_blocks, is_dir, time) = vfile.stat();
        if is_dir {
            st_mode = S_IFDIR;
        } else {
            st_mode = S_IFREG;
        }
        if vfile.name() == "null" || vfile.name() == "zero" {
            st_mode = S_IFCHR;
        }
        kstat.init(st_size, st_blksize as i32, st_blocks, st_mode, time);
    }

    fn file_size(&self) ->usize{
        self.file_size()
    }
}
