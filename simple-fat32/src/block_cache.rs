use super::{BlockDevice, BLOCK_SZ};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
#[allow(unused)]
use riscv::register::time;
use spin::RwLock; //读写锁

pub struct BlockCache {
    pub cache: [u8; BLOCK_SZ],
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
    modified: bool,
}

impl BlockCache {
    /// 从磁盘上加载一个块缓存
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SZ];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    /// 得到缓冲区中指定偏移量 offset 的字节地址
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    /// 获取缓冲区中的位于偏移量 offset 的一个类型为 T 的磁盘上数据结构的不可变引用
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }

    /// 获取缓冲区中的位于偏移量 offset 的一个类型为 T 的磁盘上数据结构的可变引用
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    /// 获取不可变引用后执行指定函数
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    /// 获取可变引用后执行指定函数
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    /// 将缓冲区中的内容写回到磁盘块中
    pub fn sync(&mut self) {
        if self.modified {
            //println!("drop cache, id = {}", self.block_id);
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

// 0-info扇区
// 1-2 FAT1
// 3-4 FAT2
// 5-7 DirEntry
// 8-19 DATA

// 双缓存：数据块和索引块，Clock算法进行淘汰
pub struct BlockCacheManager {
    start_sec: usize,
    limit: usize,
    queue: VecDeque<(usize, Arc<RwLock<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new(limit: usize) -> Self {
        Self {
            start_sec: 0,
            limit,
            queue: VecDeque::new(),
        }
    }

    pub fn set_start_sec(&mut self, new_start_sec: usize) {
        self.start_sec = new_start_sec;
    }

    pub fn get_start_sec(&self) -> usize {
        self.start_sec
    }

    // 读取指定id的块的缓存，如果不在队列里则返回None
    pub fn read_block_cache(
        &self,
        block_id: usize,
        //block_device: Arc<dyn BlockDevice>,
    ) -> Option<Arc<RwLock<BlockCache>>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Some(Arc::clone(&pair.1))
        } else {
            None
        }
    }

    // 获取一个块缓存
    pub fn get_block_cache(&mut self, block_id: usize, block_device: Arc<dyn BlockDevice>) -> Arc<RwLock<BlockCache>> {
        // 先在队列中寻找，若找到则将块缓存的引用复制一份并返回
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Arc::clone(&pair.1)
        } else {
            // 判断块缓存数量是否到达上线
            if self.queue.len() == self.limit {
                // FIFO 替换，找强引用计数为1的替换出去
                if let Some((idx, _)) = self.queue.iter().enumerate().find(|(_, pair)| Arc::strong_count(&pair.1) == 1) {
                    self.queue.drain(idx..=idx);
                } else {
                    // 队列已满且其中所有的块缓存都正在使用的情形
                    panic!("Run out of BlockCache!");
                }
            }
            // 创建新的块缓存（会触发 read_block 进行块读取）
            let block_cache = Arc::new(RwLock::new(BlockCache::new(block_id, Arc::clone(&block_device))));
            // 加入到队尾，最后返回
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }

    pub fn drop_all(&mut self) {
        self.queue.clear();
    }
}

// 1034个数据缓存块
lazy_static! {
    pub static ref DATA_BLOCK_CACHE_MANAGER: RwLock<BlockCacheManager> = RwLock::new(BlockCacheManager::new(1034));
}

// 10个信息缓存块
lazy_static! {
    pub static ref INFO_CACHE_MANAGER: RwLock<BlockCacheManager> = RwLock::new(BlockCacheManager::new(10));
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum CacheMode {
    READ,
    WRITE,
}

/* 仅用于访问文件数据块，不包括目录项 */
pub fn get_block_cache(block_id: usize, block_device: Arc<dyn BlockDevice>, rw_mode: CacheMode) -> Arc<RwLock<BlockCache>> {
    // 这里的read是RWLock读写锁
    let phy_blk_id = DATA_BLOCK_CACHE_MANAGER.read().get_start_sec() + block_id;
    if rw_mode == CacheMode::READ {
        // 判断是否在索引块缓存队列中，避免缓存一致性问题
        if let Some(blk) = INFO_CACHE_MANAGER.read().read_block_cache(phy_blk_id) {
            return blk;
        }
        // 获取块缓存，如果队列中有则直接返回，没有则申请块缓存，内置了panic
        DATA_BLOCK_CACHE_MANAGER.write().get_block_cache(phy_blk_id, block_device);
        // 返回
        DATA_BLOCK_CACHE_MANAGER.read().read_block_cache(phy_blk_id).unwrap()
    } else {
        // 这个块是要写入的
        if let Some(blk) = INFO_CACHE_MANAGER.read().read_block_cache(phy_blk_id) {
            return blk;
        }
        DATA_BLOCK_CACHE_MANAGER.write().get_block_cache(phy_blk_id, block_device)
    }
}

/* 用于访问保留扇区，以及目录项 */
pub fn get_info_cache(block_id: usize, block_device: Arc<dyn BlockDevice>, rw_mode: CacheMode) -> Arc<RwLock<BlockCache>> {
    let phy_blk_id = INFO_CACHE_MANAGER.read().get_start_sec() + block_id;
    if rw_mode == CacheMode::READ {
        // make sure the blk is in cache
        if let Some(blk) = DATA_BLOCK_CACHE_MANAGER.read().read_block_cache(phy_blk_id) {
            return blk;
        }
        INFO_CACHE_MANAGER.write().get_block_cache(phy_blk_id, block_device);
        INFO_CACHE_MANAGER.read().read_block_cache(phy_blk_id).unwrap()
    } else {
        if let Some(blk) = DATA_BLOCK_CACHE_MANAGER.read().read_block_cache(phy_blk_id) {
            return blk;
        }
        INFO_CACHE_MANAGER.write().get_block_cache(phy_blk_id, block_device)
    }
}

// 设置起始扇区
pub fn set_start_sec(start_sec: usize) {
    INFO_CACHE_MANAGER.write().set_start_sec(start_sec);
    DATA_BLOCK_CACHE_MANAGER.write().set_start_sec(start_sec);
}

// 写回磁盘，会调用Drop
pub fn write_to_dev() {
    INFO_CACHE_MANAGER.write().drop_all();
    DATA_BLOCK_CACHE_MANAGER.write().drop_all();
}
