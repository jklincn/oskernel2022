mod dirent;
mod fdset;
mod inode;
mod mount;
mod pipe;
mod stat;
mod stdio;

use crate::{mm::UserBuffer, timer::Timespec};
use alloc::{string::String, vec::Vec};

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn available(&self) -> bool;
    /// read 指的是从文件中读取数据放到缓冲区中，最多将缓冲区填满，并返回实际读取的字节数
    fn read(&self, buf: UserBuffer) -> usize;
    /// 将缓冲区中的数据写入文件，最多将缓冲区中的数据全部写入，并返回直接写入的字节数
    fn write(&self, buf: UserBuffer) -> usize;

    fn get_fstat(&self, kstat: &mut Kstat);

    fn set_time(&self, timespec: &Timespec);

    fn get_dirent(&self, dirent: &mut Dirent) -> isize;

    fn get_name(&self) -> String;

    fn get_offset(&self) -> usize;

    fn set_offset(&self, offset: usize);

    fn set_flags(&self, flag: OpenFlags);

    fn set_cloexec(&self);

    fn read_kernel_space(&self) -> Vec<u8>;

    fn write_kernel_space(&self, data: Vec<u8>) -> usize;

    fn file_size(&self) -> usize;
    
    fn r_ready(&self) -> bool {
        true
    }
    fn w_ready(&self) -> bool {
        true
    }
}

pub use dirent::Dirent;
pub use fdset::*;
pub use inode::{chdir, list_apps, open, OSInode, OpenFlags};
pub use mount::MNT_TABLE;
pub use pipe::{make_pipe, Pipe};
pub use stat::*;
pub use stdio::{Stdin, Stdout};
