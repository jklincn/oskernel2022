use super::File;
use crate::drivers::BLOCK_DEVICE;
use crate::mm::UserBuffer;
use alloc::vec::Vec;
use alloc::{string::String, sync::Arc};
use bitflags::*;
use lazy_static::*;
use simple_fat32::{FAT32Manager, VFile, ATTR_ARCHIVE, ATTR_DIRECTORY};
use spin::Mutex;

pub const SEEK_SET: i32 = 0; /* set to offset bytes.  */
pub const SEEK_CUR: i32 = 1; /* set to its current location plus offset bytes.  */
pub const SEEK_END: i32 = 2; /* set to the size of the file plus offset bytes.  */
/*  Adjust the file offset to the next location in the file
greater than or equal to offset containing data.  If
offset points to data, then the file offset is set to
offset */
pub const SEEK_DATA: i32 = 3;
/*  Adjust the file offset to the next hole in the file
greater than or equal to offset.  If offset points into
the middle of a hole, then the file offset is set to
offset.  If there is no hole past offset, then the file
offset is adjusted to the end of the file (i.e., there is
an implicit hole at the end of any file). */
pub const SEEK_HOLE: i32 = 4;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum DiskInodeType {
    File,
    Directory,
}

/// 表示进程中一个被打开的常规文件或目录
pub struct OSInode {
    readable: bool, // 该文件是否允许通过 sys_read 进行读
    writable: bool, // 该文件是否允许通过 sys_write 进行写
    inner: Mutex<OSInodeInner>,
}

pub struct OSInodeInner {
    offset: usize, // 偏移量
    inode: Arc<VFile>,
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<VFile>) -> Self {
        Self {
            readable,
            writable,
            inner: Mutex::new(OSInodeInner { offset: 0, inode }),
        }
    }
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
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

    pub fn is_dir(&self) -> bool {
        let inner = self.inner.lock();
        inner.inode.is_dir()
    }

    /* this func will not influence the file offset
     * @parm: if offset == -1, file offset will be used
     */
    pub fn read_vec(&self, offset: isize, len: usize) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut len = len;
        let ori_off = inner.offset;
        if offset >= 0 {
            inner.offset = offset as usize;
        }
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let rlen = inner.inode.read_at(inner.offset, &mut buffer);
            if rlen == 0 {
                break;
            }
            inner.offset += rlen;
            v.extend_from_slice(&buffer[..rlen.min(len)]);
            if len > rlen {
                len -= rlen;
            } else {
                break;
            }
        }
        if offset >= 0 {
            inner.offset = ori_off;
        }
        v
    }

    pub fn write_all(&self, str_vec: &Vec<u8>) -> usize {
        let mut inner = self.inner.lock();
        let mut remain = str_vec.len();
        let mut base = 0;
        loop {
            let len = remain.min(512);
            inner.inode.write_at(inner.offset, &str_vec.as_slice()[base..base + len]);
            inner.offset += len;
            base += len;
            remain -= len;
            if remain == 0 {
                break;
            }
        }
        return base;
    }
}

// 这里在实例化的时候进行文件系统的打开
lazy_static! {
    pub static ref ROOT_INODE: Arc<VFile> = {
        let fat32_manager = FAT32Manager::open(BLOCK_DEVICE.clone());
        let manager_reader = fat32_manager.read();
        Arc::new(manager_reader.create_root_vfile(&fat32_manager)) // 返回根目录
    };
}

pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls().unwrap() {
        if app.1 & ATTR_DIRECTORY == 0 {
            // 如果不是目录
            println!("{}", app.0);
        }
    }
    println!("**************/")
}

// 定义一份打开文件的标志
bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;   // 只读
        const WRONLY = 1 << 0; // 只写
        const RDWR = 1 << 1; // 可读可写
        const CREATE = 1 << 9; // 创建
        const TRUNC = 1 << 10; // 若文件存在则清空文件内容
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// 根据文件名打开一个根目录下的文件
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let path1 = String::from("./");
    let path2 = String::from(name);
    let path = path1 + &path2;
    let mut pathv: Vec<&str> = path.split('/').collect();

    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find_vfile_bypath(pathv.clone()) {
            // 如果文件已经存在，则清空文件的内容
            inode.remove();
        } //非常奇怪的一个点
        {
            // 创建文件
            ROOT_INODE
                .create(name, ATTR_ARCHIVE)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        ROOT_INODE.find_vfile_bypath(pathv).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
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
        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;
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
        let mut inner = self.inner.lock();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}
