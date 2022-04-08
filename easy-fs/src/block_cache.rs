/// # 块缓存层
/// `easy-fs/src/block_cache.rs`
/// ```
/// pub struct BlockCache
/// pub struct BlockCacheManager
/// 
/// pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager>
/// 
/// pub fn get_block_cache()
/// pub fn block_cache_sync_all()
/// ```
//

use super::{BlockDevice, BLOCK_SZ};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

/// ### 块缓存
/// |成员变量|描述|
/// |--|--|
/// |`cache`|一个 512 字节的数组，表示位于内存中的缓冲区|
/// |`block_id`|记录了这个块缓存来自于磁盘中的块的编号|
/// |`block_device`|一个底层块设备的引用，可通过它进行块读写|
/// |`modified`|记录这个块从磁盘载入内存缓存之后，它有没有被修改过|
/// 
/// ```
/// BlockCache::new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self
/// BlockCache::get_ref<T>(&self, offset: usize) -> &T
/// BlockCache::read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V
/// BlockCache::get_mut<T>(&mut self, offset: usize) -> &mut T
/// BlockCache::modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V
/// BlockCache::sync(&mut self)
/// ```
pub struct BlockCache { /// 一个 512 字节的数组，表示位于内存中的缓冲区
    cache: [u8; BLOCK_SZ],  /// 来自于磁盘中的块的编号
    block_id: usize,        /// 一个底层块设备的引用，可通过它进行块读写
    block_device: Arc<dyn BlockDevice>,/// 记录这个块从磁盘载入内存缓存之后，它有没有被修改过
    modified: bool,
}

impl BlockCache {
    /// 从磁盘读取一整个块（512Byte）上的数据到缓冲区
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

    /// 得到一个 `BlockCache` 内部的缓冲区中指定偏移量 `offset` 的字节地址中的64位数据
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    /// 泛型方法<br>它可以获取缓冲区中的位于偏移量 `offset` 的一个类型为 `T` 的磁盘上数据结构的不可变引用<br>
    /// 该泛型方法的 Trait Bound 限制类型 T 必须是一个编译时已知大小的类型
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }

    /// 它可以获取缓冲区中的位于偏移量 `offset` 的一个类型为 `T` 的磁盘上数据结构的**可变引用**
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

    /// 在 `BlockCache` 缓冲区偏移量为 `offset` 的位置获取一个类型为 `T` 的磁盘上数据结构的不可变引用，并让它执行传入的闭包 `f` 中所定义的操作
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    /// 在 `BlockCache` 缓冲区偏移量为 `offset` 的位置获取一个类型为 `T` 的磁盘上数据结构的可变引用，并让它执行传入的闭包 `f` 中所定义的操作
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    /// 缓存同步，如果缓存修改则将缓存写回磁盘
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}

/// 当 BlockCache 的生命周期结束之后缓冲区也会被从内存中回收，这个时候 modified 标记将会决定数据是否需要写回磁盘
impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

/// 内存中同时能驻留磁盘块缓冲区的上限
const BLOCK_CACHE_SIZE: usize = 16;

/// ### 块缓存管理器
/// - `queue`：双端队列，内容为二元组 `<block_id, BlockChahe>` ，用于存放磁盘块缓存
/// ```
/// BlockCacheManager::new() -> Self
/// BlockCacheManager::get_block_cache()
/// ```
pub struct BlockCacheManager {  /// 双端队列，内容为二元组 `<block_id, BlockChahe>` ，用于存放磁盘块缓存
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// 尝试从块缓存管理器中获取一个编号为 `block_id` 的块的块缓存，如果找不到，会从磁盘读取到内存中，还有可能会发生缓存替换
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Arc::clone(&pair.1) // 当我们要对一个磁盘块进行读写时，首先看它是否已经被载入到内存缓存中了，如果已经被载入的话则直接返回
            // 通过克隆使 块缓存 的强引用计数加一 用于判断块缓存是否正在被使用
        } else {
            // 内存中同时能驻留磁盘块缓冲区达到上限
            if self.queue.len() == BLOCK_CACHE_SIZE {
                // from front to tail
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                    // 强引用计数大于1表示块缓存仍在使用，不能释放
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // load block into mem and push back
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

/// 通过块缓存管理器获取块设备中的块缓存
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

/// 同步所有块缓存
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
