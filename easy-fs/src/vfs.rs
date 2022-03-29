use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

pub struct Inode {
    block_id: usize, // 保存对应的 DiskInode 在磁盘上的具体位置
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>, // 指向文件系统的指针
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// 不会在调用 Inode::new 过程中尝试获取整个 EasyFileSystem 的锁来查询 inode 在块设备中的位置，而是在调用它之前预先查询并作为参数传过去。
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    // 简化对于 Inode 对应的磁盘上的 DiskInode 的访问流程，传入一个函数
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    /// 尝试从根目录的 DiskInode 上找到要索引的文件名对应的 inode 编号
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // 判断是否为目录
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ; // 该目录下总共有多少个文件
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                // 判断读取是否正确
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32); //返回实际 inode 编号
            }
        }
        None
    }

    /// 只会被根目录 Inode 调用
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                // 把 Option<u32> 转为 Option<Arc<Inode>>
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id); // 传入 inode 编号查找实际位置
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        // 分配新的数据块
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        // 把新数据块向量作为参数传入
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// 在根目录下创建一个文件，该方法只有根目录的 Inode 会调用
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self
            .modify_disk_inode(|root_inode: &mut DiskInode| {
                // 判断是否是目录
                assert!(root_inode.is_dir());
                // 文件是否已创建
                self.find_inode_id(name, root_inode)
            })
            .is_some()
        {
            // 根目录下已有文件，直接返回
            return None;
        }
        // 为待创建文件分配一个新的 inode 并进行初始化
        let new_inode_id = fs.alloc_inode();
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        // 将待创建文件的目录项插入到根目录的内容中
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // 扩充大小
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // 新建目录项并写入根目录的内容中
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    /// 收集根目录下的所有文件的文件名并以向量的形式返回
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock(); // _表明我们仅仅是锁住该实例避免其他核在同时间的访问造成并发冲突，而并没有使用它，这样编译器不会报警告
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                // 加入到向量中
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    /// 读取内容
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// 写入内容
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    /// 文件清空
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size; // 获取当前文件的总字节数
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device); // 返回一个 u32 向量
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize); // 判断清除的块数是否和这个文件所需的全部块数相等
            for data_block in data_blocks_dealloc.into_iter() {
                // 依次回收数据块
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
