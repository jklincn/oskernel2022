use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;

/// 磁盘数据结构，它将位图区域中的一个磁盘块解释为长度为 64 的一个 u64 数组，
/// 每个 u64 打包了一组 64 bits，于是整个数组包含 64 * 64 = 4096 bits，且可以以组为单位进行操作
type BitmapBlock = [u64; 64];

const BLOCK_BITS: usize = BLOCK_SZ * 8;

/// 位图结构体，每个块大小为 512 bytes，即 4096 bits，每个 bit 都代表一个索引节点/数据块的分配状态
pub struct Bitmap {
    start_block_id: usize, // 所在区域的起始块编号
    blocks: usize,         // 区域的长度为多少个块
}

/// 将bit编号 bit 分解为区域中的块编号 block_pos 、块内的组编号 bits64_pos 以及组内编号 inner_pos 的三元组
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    /// 新建一个位图
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// 分配一个bit
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        // 枚举自身区域中的每个块
        for block_id in 0..self.blocks {
            // 获取块缓存
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock() // 获取块缓存的互斥锁从而可以对块缓存进行访问
            // 0 是 modify 方法中的偏移量参数，这是因为整个块上只有一个 BitmapBlock ，它的大小恰好为 512 字节。因此我们需要从块的开头开始才能访问到完整的 BitmapBlock
            // 传给它的闭包需要显式声明参数类型为 &mut BitmapBlock ，不然的话， BlockCache 的泛型方法 modify/get_mut 无法得知应该用哪个类型来解析块上的数据。
            // 这里 modify 的含义就是：从缓冲区偏移量为 0 的位置开始将一段连续的数据（数据的长度随具体类型而定）解析为一个 BitmapBlock 并要对该数据结构进行修改。
            // 在闭包内部，我们可以使用这个 BitmapBlock 的可变引用 bitmap_block 对它进行访问。
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // 遍历每 64 bits构成的组（一个 u64 ），如果它并没有达到 u64::MAX, 则通过 u64::trailing_ones 找到最低的一个 0 并置为 1
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX) // 若找到，则返回 bit 组的编号与组内偏移量
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))// 计算1的数量再类型转换
                {
                    // 这里是 if 为 true 的语句
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

    /// 回收一个bit
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit); // 定位待回收的 bit
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }

    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
