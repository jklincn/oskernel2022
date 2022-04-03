mod inode;
mod pipe;
mod stdio;

/// UserBuffer 是我们在 mm 子模块中定义的应用地址空间中的一段缓冲区（即内存）的抽象，本质上是一个 &[u8]
use crate::mm::UserBuffer;

/// 一切皆是文件
pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}

pub use inode::{list_apps, open_file, OSInode, OpenFlags};
pub use pipe::{make_pipe, Pipe};
pub use stdio::{Stdin, Stdout};
