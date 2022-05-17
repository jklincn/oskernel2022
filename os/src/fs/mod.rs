/// # 内核文件系统接口
/// `os/src/fs/mod.rs`
//

mod inode;  // 内核索引节点层
mod stdio;  // 标准输入输出接口
mod pipe;   // 管道模块
mod mount;  // 挂载模块
mod stat;
mod dirent;

use crate::mm::UserBuffer;
use alloc::string::String;

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    /// read 指的是从文件中读取数据放到缓冲区中，最多将缓冲区填满，并返回实际读取的字节数
    fn read(&self, buf: UserBuffer) -> usize;
    /// 将缓冲区中的数据写入文件，最多将缓冲区中的数据全部写入，并返回直接写入的字节数
    fn write(&self, buf: UserBuffer) -> usize;

    fn get_fstat(&self, kstat:&mut Kstat);

    fn get_dirent(&self, dirent: &mut Dirent)->isize;

    fn get_name(&self) -> String;
}


pub use inode::{list_apps, open, OSInode, OpenFlags,chdir};
pub use stdio::{Stdin, Stdout};
pub use pipe::{make_pipe, Pipe};
pub use mount::MNT_TABLE;
pub use stat::Kstat;
pub use dirent::Dirent;
