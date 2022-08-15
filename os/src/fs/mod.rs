mod dirent;
mod fdset;
mod inode;
mod mount;
mod pipe;
mod stat;
mod stdio;

use crate::{mm::UserBuffer, timer::Timespec};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn available(&self) -> bool;
    /// read 指的是从文件中读取数据放到缓冲区中，最多将缓冲区填满，并返回实际读取的字节数
    fn read(&self, buf: UserBuffer) -> usize;
    /// 将缓冲区中的数据写入文件，最多将缓冲区中的数据全部写入，并返回直接写入的字节数
    fn write(&self, buf: UserBuffer) -> usize;

    fn get_name(&self) -> &str;

    fn get_fstat(&self, _kstat: &mut Kstat) {
        panic!("{} not implement get_fstat", self.get_name());
    }

    fn set_time(&self, _timespec: &Timespec) {
        panic!("{} not implement set_time", self.get_name());
    }

    fn get_dirent(&self, _dirent: &mut Dirent) -> isize {
        panic!("{} not implement get_dirent", self.get_name());
    }

    fn get_path(&self) -> &str {
        panic!("{} not implement get_path", self.get_name());
    }

    fn get_offset(&self) -> usize {
        panic!("{} not implement get_offset", self.get_name());
    }

    fn set_offset(&self, _offset: usize) {
        panic!("{} not implement set_offset", self.get_name());
    }

    fn set_flags(&self, _flag: OpenFlags) {
        panic!("{} not implement set_flags", self.get_name());
    }

    fn set_cloexec(&self) {
        panic!("{} not implement set_cloexec", self.get_name());
    }

    fn read_kernel_space(&self) -> Vec<u8> {
        panic!("{} not implement read_kernel_space", self.get_name());
    }

    fn write_kernel_space(&self, _data: Vec<u8>) -> usize {
        panic!("{} not implement write_kernel_space", self.get_name());
    }

    fn file_size(&self) -> usize {
        panic!("{} not implement file_size", self.get_name());
    }

    fn r_ready(&self) -> bool {
        true
    }
    fn w_ready(&self) -> bool {
        true
    }
}

// impl Debug for dyn File {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         f.write_fmt(format_args!("File trait"))
//     }
// }

impl Debug for dyn File + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("name:{}", self.get_name()))
    }
}

pub use dirent::Dirent;
pub use fdset::*;
pub use inode::{chdir, init, open, OSInode, OpenFlags};
pub use mount::MNT_TABLE;
pub use pipe::{make_pipe, Pipe};
pub use stat::*;
pub use stdio::{Stdin, Stdout};
