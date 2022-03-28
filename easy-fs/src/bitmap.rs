/// # 位示图
/// `easy-fs/src/bitmap.rs`
/// ```
/// pub struct Bitmap
/// ```
//

use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;

/// 磁盘数据结构，它将位图区域中的一个磁盘块解释为长度为 64 的一个 `u64` 数组，4096 bits
type BitmapBlock = [u64; 64];

/// 块大小字节数 4096 bits
const BLOCK_BITS: usize = BLOCK_SZ * 8;

/// ### 位示图
/// 记录一片区域（所在区域的起始块编号以及区域的长度）磁盘块的使用情况
/// - `start_block_id`: 区域的起始块编号
/// - `blocks`: 区域的长度
/// ```
/// Bitmap::new(start_block_id: usize, blocks: usize) -> Self
/// Bitmap::alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize>
/// Bitmap::dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize)
/// Bitmap::maximum(&self) -> usize
/// ```
pub struct Bitmap { /// 区域的起始块编号
    start_block_id: usize,  /// 区域的长度
    blocks: usize,
}

/// 将 bit 编号分解为 `区域中的块编号 block_pos` 、`块内的组编号 bits64_pos` 以及 `组内编号 inner_pos` 的三元组
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// 从位示图中找出一个可用的物理块，将其标记为已使用，并返回物理块位置
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {    // 枚举区域中的每个块
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {   // 定义了一个闭包函数
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX)
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))// 找到最低的一个 0
                {
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                } else {
                    None
                }
            });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    /// 释放将 `bit` 所指的磁盘块
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }

    /// 获取位置图能表示的最大空间 单位：bit
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
