#![no_std]

extern crate alloc;

mod bitmap;     // 位示图
mod block_cache;// 块缓存层
mod block_dev;  // 块设备接口层
mod efs;        // 磁盘块管理
mod layout;     // 文件系统结构
mod vfs;        // 索引节点

/// 块大小 512字节
pub const BLOCK_SZ: usize = 512;

use bitmap::Bitmap;
use block_cache::{block_cache_sync_all, get_block_cache};
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
use layout::*;
pub use vfs::Inode;
