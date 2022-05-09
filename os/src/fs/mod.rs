/// # 内核文件系统接口
/// `os/src/fs/mod.rs`
//

mod inode;  // 内核索引节点层
mod stdio;  // 标准输入输出接口
mod pipe;   // 管道模块

use crate::mm::UserBuffer;

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    /// read 指的是从文件中读取数据放到缓冲区中，最多将缓冲区填满，并返回实际读取的字节数
    fn read(&self, buf: UserBuffer) -> usize;
    /// 将缓冲区中的数据写入文件，最多将缓冲区中的数据全部写入，并返回直接写入的字节数
    fn write(&self, buf: UserBuffer) -> usize;
}

pub use inode::{list_apps, open, OSInode, OpenFlags};
pub use stdio::{Stdin, Stdout};
pub use pipe::{make_pipe, Pipe};
