/// # 文件系统结构
/// `easy-fs/src/layout.rs`
/// ```
/// pub struct SuperBlock
/// pub enum DiskInodeType
/// ```
//

use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result};

const EFS_MAGIC: u32 = 0x3b800001;
const INODE_DIRECT_COUNT: usize = 28;
const NAME_LENGTH_LIMIT: usize = 27;
const INODE_INDIRECT1_COUNT: usize = BLOCK_SZ / 4;
const INODE_INDIRECT2_COUNT: usize = INODE_INDIRECT1_COUNT * INODE_INDIRECT1_COUNT;
const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_INDIRECT1_COUNT;
#[allow(unused)]
const INDIRECT2_BOUND: usize = INDIRECT1_BOUND + INODE_INDIRECT2_COUNT;

/// ### 超级块
/// |成员变量|描述|
/// |--|--|
/// |`magic`|一个用于文件系统合法性验证的魔数|
/// |`total_blocks`|文件系统的总块数|
/// |`inode_bitmap_blocks`|索引节点位示图块数|
/// |`inode_area_blocks`|索引节点块数|
/// |`data_bitmap_blocks`|数据节点位示图块数|
/// |`data_area_blocks`|数据节点块数|
/// ```
/// SuperBlock::initialize()
/// SuperBlock::is_valid(&self) -> bool
/// ```
#[repr(C)]
pub struct SuperBlock { /// 一个用于文件系统合法性验证的魔数
    magic: u32, /// 文件系统的总块数
    pub total_blocks: u32,  /// 索引节点位示图块数
    pub inode_bitmap_blocks: u32,   /// 索引节点块数
    pub inode_area_blocks: u32,     /// 数据节点位示图块数
    pub data_bitmap_blocks: u32,    /// 数据节点块数
    pub data_area_blocks: u32,
}

impl Debug for SuperBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("SuperBlock")
            .field("total_blocks", &self.total_blocks)
            .field("inode_bitmap_blocks", &self.inode_bitmap_blocks)
            .field("inode_area_blocks", &self.inode_area_blocks)
            .field("data_bitmap_blocks", &self.data_bitmap_blocks)
            .field("data_area_blocks", &self.data_area_blocks)
            .finish()
    }
}

impl SuperBlock {
    /// 对超级块进行初始化，注意各个区域的块数是以参数的形式传入进来的，它们的划分是更上层的磁盘块管理器需要完成的工作
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }
    /// 通过魔数判断超级块所在的文件系统是否合法
    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

type IndirectBlock = [u32; BLOCK_SZ / 4];
type DataBlock = [u8; BLOCK_SZ];


