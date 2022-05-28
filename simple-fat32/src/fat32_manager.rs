use super::{get_block_cache, set_start_sec, write_to_dev, BlockDevice, FSInfo, FatBS, FatExtBS, FAT};
use crate::layout::*;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

pub struct FAT32Manager {
    block_device: Arc<dyn BlockDevice>,       // 块设备的引用
    fsinfo: Arc<RwLock<FSInfo>>,              // 文件系统信息扇区的引用
    sectors_per_cluster: u32,                 // 每个簇的扇区数
    bytes_per_sector: u32,                    // 每个扇区的字节数
    bytes_per_cluster: u32,                   // 每个簇的字节数
    fat: Arc<RwLock<FAT>>,                    // FAT的引用
    root_sec: u32,                            // 根目录所在簇号
    vroot_dirent: Arc<RwLock<ShortDirEntry>>, // 虚拟根目录项。根目录无目录项，引入以与其他文件一致
}

impl FAT32Manager {
    pub fn sectors_per_cluster(&self) -> u32 {
        self.sectors_per_cluster
    }

    pub fn bytes_per_sector(&self) -> u32 {
        self.bytes_per_sector
    }

    pub fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_cluster
    }

    pub fn block_device(&self) -> Arc<dyn BlockDevice> {
        self.block_device.clone()
    }

    /// 第一个数据簇（ROOT）的扇区
    pub fn first_data_sector(&self) -> u32 {
        self.root_sec
    }

    /// 从 fsinfo 中读取空闲簇的数量
    pub fn free_clusters(&self) -> u32 {
        self.fsinfo.read().free_clusters()
    }

    /* 某个簇的第一个扇区 */
    pub fn first_sector_of_cluster(&self, cluster: u32) -> usize {
        (cluster as usize - 2) * self.sectors_per_cluster as usize + self.root_sec as usize
    }

    /* 打开现有的FAT32  */
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>> {
        // 读入分区偏移
        let start_sector: u32 = get_block_cache(0, Arc::clone(&block_device))
            .read()
            .read(0x1c6, |ssec_bytes: &[u8; 4]| {
                // 0x1c6可以看文档
                let mut start_sec: u32 = 0;
                // 小端存储
                for i in 0..4 {
                    let tmp = ssec_bytes[i] as u32;
                    start_sec = start_sec + (tmp << (8 * i));
                }
                start_sec
            });

        // 设置分区偏移
        set_start_sec(start_sector as usize);

        // 读入第一分区的 Boot Sector，get_info_cache方法中会自动加上分区偏移
        let boot_sec = get_block_cache(0, Arc::clone(&block_device)).read().read(0, |bs: &FatBS| *bs);

        // 读入 Extended Boot Sector
        let ext_boot_sec = get_block_cache(0, Arc::clone(&block_device)).read().read(36, |ebs: &FatExtBS| *ebs);

        // 创建 fsinfo
        let fsinfo = get_block_cache(ext_boot_sec.fat_info_sec() as usize, Arc::clone(&block_device))
            .read()
            .read(0, |fsinfo: &FSInfo| *fsinfo);

        // 校验签名
        assert!(fsinfo.check_signature(), "Error loading fat32! Illegal signature");

        let sectors_per_cluster = boot_sec.sec_per_clus(); // 每簇包含的扇区数
        let bytes_per_sector = boot_sec.bytes_per_sec(); // 每扇区包含的字节数
        let bytes_per_cluster = sectors_per_cluster * bytes_per_sector; // 每簇包含的字节数
        let fat_n_sec = ext_boot_sec.fat_size(); // fat表所占的扇区数，即fat表大小
        let fat1_sector = boot_sec.rsvd_sec_cnt(); // fat表1起始的扇区号
        let fat2_sector = fat1_sector + fat_n_sec; // fat表2起始的扇区号

        let fat = FAT::new(fat1_sector, fat2_sector);

        // 保留扇区数 + 所有FAT表的扇区数
        let root_sec = boot_sec.rsvd_sec_cnt() + boot_sec.fat_num() * fat_n_sec;

        // 0x2F in ASCII is /
        let mut root_dirent = ShortDirEntry::new();
        root_dirent.initialize(
            &[0x2F, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20],
            &[0x20, 0x20, 0x20],
            ATTR_DIRECTORY,
        );

        root_dirent.set_first_cluster(2);

        let fat32_manager = Self {
            block_device,
            fsinfo: Arc::new(RwLock::new(fsinfo)),
            sectors_per_cluster,
            bytes_per_sector,
            bytes_per_cluster,
            fat: Arc::new(RwLock::new(fat)),
            root_sec,
            vroot_dirent: Arc::new(RwLock::new(root_dirent)),
        };

        Arc::new(RwLock::new(fat32_manager))
    }

    /// 获取（虚拟）根目录项
    pub fn get_root_dirent(&self) -> Arc<RwLock<ShortDirEntry>> {
        self.vroot_dirent.clone()
    }

    /// 分配簇，会填写 FAT，成功返回第一个簇号，失败返回 None
    // TODO:分配的时候清零
    pub fn alloc_cluster(&self, num: u32) -> Option<u32> {
        let free_clusters = self.free_clusters();
        // 没有足够的空闲簇
        if num > free_clusters {
            return None;
        }
        // 读取最后被分配的簇号
        let prev_cluster = self.fsinfo.read().next_free_cluster();
        // 获得第一个空闲簇
        let first_cluster: u32 = self.fat.write().get_free_cluster(prev_cluster, self.block_device.clone());
        let mut current_cluster = first_cluster;
        // 如果 num 大于 1 则继续搜索可用簇，同时写表项
        for _ in 1..num {
            // 清空当前簇的内容
            self.clear_cluster(current_cluster);
            // 获取空闲簇号
            let next_cluster = self.fat.write().get_free_cluster(current_cluster, self.block_device.clone());
            assert_ne!(next_cluster, 0);
            // 设置 FAT 表
            self.fat
                .write()
                .set_next_cluster(current_cluster, next_cluster, self.block_device.clone());
            current_cluster = next_cluster;
        }
        self.clear_cluster(current_cluster);
        // 填写最后一个表项
        self.fat
            .write()
            .set_next_cluster(current_cluster, END_CLUSTER, self.block_device.clone());
        // 更新空闲簇的数量
        self.fsinfo.write().set_free_clusters(free_clusters - num);
        // 更新分配的最后一个簇
        self.fsinfo.write().set_next_free_cluster(current_cluster);
        Some(first_cluster)
    }

    pub fn dealloc_cluster(&self, clusters: Vec<u32>) {
        let free_clusters = self.free_clusters();
        let num = clusters.len();
        for i in 0..num {
            // 将FAT对应表项清零
            self.fat
                .write()
                .set_next_cluster(clusters[i], FREE_CLUSTER, self.block_device.clone())
        }
        // 修改 FSINFO
        if num > 0 {
            self.fsinfo.write().set_free_clusters(free_clusters + num as u32);
            // 如果释放的簇号小于开始空闲簇字段，更新该字段
            if clusters[0] > 2 && clusters[0] < self.fsinfo.read().next_free_cluster() {
                self.fsinfo.write().set_next_free_cluster(clusters[0] - 1);
            }
        }
    }

    // 清空簇的内容（全部写 0）
    fn clear_cluster(&self, cluster_id: u32) {
        let start_sec = self.first_sector_of_cluster(cluster_id);
        for i in 0..self.sectors_per_cluster {
            get_block_cache(start_sec + i as usize, self.block_device.clone())
                .write()
                .modify(0, |blk: &mut [u8; 512]| {
                    for byte in blk.iter_mut() {
                        *byte = 0;
                    }
                });
        }
    }

    /// 获取 FAT
    pub fn get_fat(&self) -> Arc<RwLock<FAT>> {
        Arc::clone(&self.fat)
    }

    /// 字节转化为所需的簇数
    fn size_to_clusters(&self, size: u32) -> u32 {
        (size + self.bytes_per_cluster - 1) / self.bytes_per_cluster
    }

    /* 计算扩大至 new_size 需要多少个簇 */
    pub fn cluster_num_needed(&self, old_size: u32, new_size: u32, is_dir: bool, first_cluster: u32) -> u32 {
        // 对于目录的不同计算方法需要考虑目录数据排布问题 todo
        if is_dir {
            // 计算簇的数量
            let old_clusters = self.fat.read().count_claster_num(first_cluster, self.block_device.clone());
            self.size_to_clusters(new_size) - old_clusters
        } else {
            self.size_to_clusters(new_size) - self.size_to_clusters(old_size)
        }
    }

    // 目前没有数据持久化的需求
    #[allow(unused)]
    pub fn cache_write_back(&self) {
        write_to_dev();
    }
}

/// 将长文件名拆分，返回字符串数组
pub fn long_name_split(name: &str) -> Vec<String> {
    let len = name.len() as u32; // 要有\0
    let name_bytes = name.as_bytes();
    let mut name_vec: Vec<String> = Vec::new();
    // 计算需要几个目录项，向上取整
    // 以 13个字符为单位进行切割，每一组占据一个目录项
    let n_ent = (len + LONG_NAME_LEN - 1) / LONG_NAME_LEN;
    let mut temp_buffer = String::new();
    // 如果文件名结束但还有未使用的字节，则会在文件名后先填充两个字节的 0x00，然后开始使用 0xFF 填充
    for i in 0..n_ent {
        temp_buffer.clear();
        for j in i * LONG_NAME_LEN..(i + 1) * LONG_NAME_LEN {
            if j < len {
                // 有效的文件名字
                temp_buffer.push(name_bytes[j as usize] as char);
            } else if j > len {
                temp_buffer.push(0xFF as char);
            } else {
                temp_buffer.push(0x00 as char);
            }
        }
        name_vec.push(temp_buffer.clone());
    }
    name_vec
}

/// 拆分文件名和后缀
pub fn split_name_ext(name: &str) -> (&str, &str) {
    match name {
        "." => return (".", ""),
        ".." => return ("..", ""),
        _ => {
            let mut name_and_ext: Vec<&str> = name.split(".").collect(); // 按 . 进行分割
            if name_and_ext.len() == 1 {
                // 如果没有后缀名则推入一个空值
                name_and_ext.push("");
            }
            (name_and_ext[0], name_and_ext[1])
        }
    }
}

/// 将短文件名格式化为目录项存储的内容
pub fn short_name_format(name: &str) -> ([u8; 8], [u8; 3]) {
    let (name, ext) = split_name_ext(name);
    let name_bytes = name.as_bytes();
    let ext_bytes = ext.as_bytes();
    let mut f_name = [0u8; 8];
    let mut f_ext = [0u8; 3];
    for i in 0..8 {
        if i >= name_bytes.len() {
            f_name[i] = 0x20; // 不足的用0x20进行填充
        } else {
            f_name[i] = (name_bytes[i] as char).to_ascii_uppercase() as u8;
        }
    }
    for i in 0..3 {
        if i >= ext_bytes.len() {
            f_ext[i] = 0x20; // 不足的用0x20进行填充
        } else {
            f_ext[i] = (ext_bytes[i] as char).to_ascii_uppercase() as u8;
        }
    }
    (f_name, f_ext)
}

/// 由长文件名生成短文件名
pub fn generate_short_name(long_name: &str) -> String {
    // 目前仅支持【name.extension】 或者 【没有后缀】 形式的文件名！
    let (name_, ext_) = split_name_ext(long_name);
    let name = name_.as_bytes();
    let extension = ext_.as_bytes();
    let mut short_name = String::new();
    // 取长文件名的前6个字符加上"~1"形成短文件名，扩展名不变，
    // 目前不支持重名，即"~2""~3"
    for i in 0..6 {
        short_name.push((name[i] as char).to_ascii_uppercase())
    }
    short_name.push('~');
    short_name.push('1');
    let ext_len = extension.len();
    for i in 0..3 {
        //fill extension
        if i >= ext_len {
            short_name.push(0x20 as char); // 不足的用0x20进行填充
        } else {
            short_name.push((name[i] as char).to_ascii_uppercase());
        }
    }
    // 返回一个长度为 11 的string数组
    short_name
}
