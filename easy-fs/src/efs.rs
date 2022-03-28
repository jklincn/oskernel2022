use super::{
    block_cache_sync_all, get_block_cache, Bitmap, BlockDevice, DiskInode, DiskInodeType, Inode,
    SuperBlock,
};
use crate::BLOCK_SZ;
use alloc::sync::Arc;
use spin::Mutex;

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,  // 块设备指针，dyn：编译时还不知道的实现了 BlockDevice 的结构体
    pub inode_bitmap: Bitmap,   // 索引节点位图
    pub data_bitmap: Bitmap,   // 数据块位图
    inode_area_start_block: u32,  // 索引节点区域起始块编号
    data_area_start_block: u32,  // 数据块区域起始块编号
}

type DataBlock = [u8; BLOCK_SZ];

impl EasyFileSystem {
    /// 在块设备上创建并初始化一个 easy-fs 文件系统
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,   // 索引节点位图块数
    ) -> Arc<Mutex<Self>> {
        // 计算每个区域各应该包含多少块
        // 在此我们假设 有 8GiB 的外部存储，则总共有 8*1024*1024*1024/512 = 16777216 个块
        // 在此我们假设 inode_bitmap_blocks = 20，即索引节点位图需要 20 个块来存
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize); // 起始块为 1 ，第 0 块为超级块
        let inode_num = inode_bitmap.maximum();  // inode_bitmap_blocks * 4096，即有 81920 个索引节点
        let inode_area_blocks =  // 计算存放索引节点的块数，(81920*128+512-1)/512 = 20480；另一种算法：一个块可以存放 4 个索引节点，因此 81920/4 = 20480
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks; // 所以此时索引节点区域的总块数为 20+20480 = 20500 块
        let data_total_blocks = total_blocks - 1 - inode_total_blocks; // 剩余的块全部用于存放数据，即 16777216 - 20500 = 16756716 块
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097; // 剩余的块除以 4097 再向上取整作为存放位图的块，即 4090 ，即数据块位图需要 4090 个块来存放
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;// 则可以得出实际用于存放数据的有 16756716 - 4090 = 16752626 块，即 7.98827 GiB
        let data_bitmap = Bitmap::new(
            (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        // 创建文件系统实例
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // 对所有要用到的块进行清零
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // 初始化超级块（第 0 块）
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        // 创建根目录 /
        assert_eq!(efs.alloc_inode(), 0);  // 第一次分配，编号一定是 0 
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);  // 根据 inode 编号获取该 inode 所在的块的编号以及块内偏移
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }

    /// 从一个已写入了 easy-fs 镜像的块设备上打开 easy-fs
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        // 读出超级块内容
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }

    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        // acquire efs lock temporarily
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        // release efs lock
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }

    /// 计算存储 inode 的磁盘块在磁盘上的实际位置
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();   // 128B
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32; // 4
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * inode_size,  // 一个块中可以存 4 个索引节点，这个返回值代表第几个
        )
    }

    /// 计算存储数据的磁盘块在磁盘上的实际位置
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }

    /// inode 的分配
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// 返回的参数都表示数据块在块设备上的编号，而不是在数据块位图中分配的bit编号
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    /// 回收数据块
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        // 在数据位图中置 0
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }
}
