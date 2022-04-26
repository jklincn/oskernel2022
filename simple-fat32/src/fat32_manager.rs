use super::{get_block_cache, get_info_cache, set_start_sec, write_to_dev, BlockDevice, FSInfo, FatBS, FatExtBS, FAT};
use alloc::sync::Arc;
//#[macro_use]
use crate::{layout::*, VFile};
use alloc::string::String;
use alloc::vec::Vec;
use spin::RwLock;
//use console;

pub struct FAT32Manager {
    block_device: Arc<dyn BlockDevice>, // 块设备的引用
    fsinfo: Arc<FSInfo>,                // 文件系统信息扇区的引用
    sectors_per_cluster: u32,           // 每个簇的扇区数
    bytes_per_sector: u32,              // 每个扇区的字节数
    bytes_per_cluster: u32,             // 每个簇的字节数
    fat: Arc<RwLock<FAT>>,              // FAT的引用
    root_sec: u32,                      // 根目录所在簇号
    #[allow(unused)]
    total_sectors: u32, //总扇区数
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

    /* 第一个数据簇（ROOT）的扇区 */
    pub fn first_data_sector(&self) -> u32 {
        self.root_sec
    }

    /* 某个簇的第一个扇区 */
    pub fn first_sector_of_cluster(&self, cluster: u32) -> usize {
        //println!("first_sector_of_cluster: cluster = {}", cluster);
        (cluster as usize - 2) * self.sectors_per_cluster as usize + self.root_sec as usize
    }

    /* 打开现有的FAT32  */
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>> {
        // 读入分区偏移
        let start_sector: u32 = get_info_cache(0, Arc::clone(&block_device))
            .read()
            .read(0x1c6, |ssec_bytes: &[u8; 4]| {
                //0x1c6可以看文档
                // DEBUG
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
        let boot_sec: FatBS = get_info_cache(0, Arc::clone(&block_device))
            .read()
            .read(0, |bs: &FatBS| {
                // DEBUG
                *bs
            });

        // 读入 Extended Boot Sector
        let ext_boot_sec: FatExtBS = get_info_cache(0, Arc::clone(&block_device))
            .read()
            .read(36, |ebs: &FatExtBS| {
                *ebs // DEBUG
            });

        // 获取 FSINFO 结构所在扇区号（通常是1），创建 fsinfo
        let fsinfo = FSInfo::new(ext_boot_sec.fat_info_sec());

        // 校验签名
        assert!(
            fsinfo.check_signature(Arc::clone(&block_device)),
            "Error loading fat32! Illegal signature"
        );

        let sectors_per_cluster = boot_sec.sec_per_clus() as u32; // 每簇包含的扇区数
        let bytes_per_sector = boot_sec.bytes_per_sec() as u32; // 每扇区包含的字节数
        let bytes_per_cluster = sectors_per_cluster * bytes_per_sector; // 每簇包含的字节数
        let fat_n_sec = ext_boot_sec.fat_size(); // fat表所占的扇区数，即fat表大小
        let fat1_sector = boot_sec.first_fat_sector() as u32; // fat表1起始的扇区号
        let fat2_sector = fat1_sector + fat_n_sec; // fat表2起始的扇区号
        let fat_n_entry = fat_n_sec * bytes_per_sector / 4; // fat（最大支持？）表项数量

        // 在内存的数据结构
        let fat = FAT::new(fat1_sector, fat2_sector, fat_n_sec, fat_n_entry);

        // 保留扇区数+所有FAT表的扇区数
        let root_sec = boot_sec.first_fat_sector() as u32 + boot_sec.fat_num() as u32 * fat_n_sec;

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
            fsinfo: Arc::new(fsinfo),
            sectors_per_cluster,
            bytes_per_sector,
            bytes_per_cluster,
            fat: Arc::new(RwLock::new(fat)),
            root_sec,
            total_sectors: boot_sec.total_sectors(),
            vroot_dirent: Arc::new(RwLock::new(root_dirent)),
        };

        Arc::new(RwLock::new(fat32_manager))
    }

    pub fn get_root_vfile(&self, fs_manager: &Arc<RwLock<Self>>) -> VFile {
        let long_pos_vec: Vec<(usize, usize)> = Vec::new();
        VFile::new(
            String::from("/"),
            0,
            0,
            long_pos_vec,
            ATTR_DIRECTORY,
            Arc::clone(fs_manager),
            self.block_device.clone(),
        )
    }

    /// 获取（虚拟）根目录项
    pub fn get_root_dirent(&self) -> Arc<RwLock<ShortDirEntry>> {
        self.vroot_dirent.clone()
    }

    /* 分配簇，会填写FAT，成功返回第一个簇号，失败返回None */
    // TODO:分配的时候清零
    pub fn alloc_cluster(&self, num: u32) -> Option<u32> {
        let free_clusters = self.free_clusters();
        if num > free_clusters {
            return None;
        }
        // 获取FAT写锁
        let fat_writer = self.fat.write();
        let prev_cluster = self.fsinfo.first_free_cluster(self.block_device.clone());
        //fat_writer.set_next_cluster(current_cluster, next_cluster, self.block_device.clone());
        //let mut cluster_vec:Vec<u32> = Vec::new();
        //cluster_vec.push(current_cluster);
        //let first_cluster = current_cluster;
        let first_cluster: u32 = fat_writer.next_free_cluster(prev_cluster, self.block_device.clone());
        let mut current_cluster = first_cluster;
        //println!("alloc: first = {}, num = {}", first_cluster, num);
        // 搜索可用簇，同时写表项
        #[allow(unused)]
        for i in 1..num {
            self.clear_cluster(current_cluster);
            let next_cluster = fat_writer.next_free_cluster(current_cluster, self.block_device.clone());
            assert_ne!(next_cluster, 0);
            fat_writer.set_next_cluster(current_cluster, next_cluster, self.block_device.clone());
            //cluster_vec.push(next_cluster);
            //println!("alloc: next = {}", fat_writer.get_next_cluster(current_cluster, self.block_device.clone()));
            current_cluster = next_cluster;
        }
        self.clear_cluster(current_cluster);
        // 填写最后一个表项
        fat_writer.set_end(current_cluster, self.block_device.clone());
        // 修改FSINFO
        //let next_cluster = fat_writer.next_free_cluster(current_cluster, self.block_device.clone());
        self.fsinfo.write_free_clusters(free_clusters - num, self.block_device.clone());
        // 写入分配的最后一个簇
        self.fsinfo.write_first_free_cluster(current_cluster, self.block_device.clone());
        //self.cache_write_back();
        //println!("[fs]: after alloc, first free cluster = {}",self.fsinfo.first_free_cluster(self.block_device.clone()));
        Some(first_cluster)
    }

    pub fn dealloc_cluster(&self, clusters: Vec<u32>) {
        let fat_writer = self.fat.write();
        let free_clusters = self.free_clusters();
        let num = clusters.len();
        for i in 0..num {
            // 将FAT对应表项清零
            fat_writer.set_next_cluster(clusters[i], FREE_CLUSTER, self.block_device.clone())
        }
        // 修改FSINFO
        if num > 0 {
            self.fsinfo
                .write_free_clusters(free_clusters + num as u32, self.block_device.clone());
            // 如果释放的簇号小于开始空闲簇字段，更新该字段
            if clusters[0] > 2 && clusters[0] < self.fsinfo.first_free_cluster(self.block_device.clone()) {
                self.fsinfo.write_first_free_cluster(clusters[0] - 1, self.block_device.clone());
            }
        }
        //println!("[fs]: after dealloc, first free cluster = {}",self.fsinfo.first_free_cluster(self.block_device.clone()));
    }

    pub fn clear_cluster(&self, cluster_id: u32) {
        let start_sec = self.first_sector_of_cluster(cluster_id);
        for i in 0..self.sectors_per_cluster {
            get_block_cache(start_sec + i as usize, self.block_device.clone())
                .write()
                .modify(0, |blk: &mut [u8; 512]| {
                    for j in 0..512 {
                        blk[j] = 0;
                    }
                });
        }
    }

    /// 获取 FAT
    pub fn get_fat(&self) -> Arc<RwLock<FAT>> {
        Arc::clone(&self.fat)
    }

    /* 获取vfs的文件对象 */
    /*
    pub fn get_vfile(){
        // TODO
    }*/

    /* 计算扩大至new_size(B)需要多少个簇 */
    pub fn cluster_num_needed(&self, old_size: u32, new_size: u32, is_dir: bool, first_cluster: u32) -> u32 {
        if old_size >= new_size {
            0
        } else {
            if is_dir {
                //println!("count old_clusters");
                let old_clusters = self.fat.read().count_claster_num(first_cluster, self.block_device.clone());
                //println!("first cluster = {}, old_clusters = {}, new_clusters = {}", first_cluster, old_clusters, self.size_to_clusters(new_size));
                // DEBUG 这里有问题 ?
                self.size_to_clusters(new_size) - old_clusters
            } else {
                self.size_to_clusters(new_size) - self.size_to_clusters(old_size)
            }
            //println!("oldsz = {}; new_sz = {}", old_size, new_size);
        }
    }

    /// 字节转化为所需的簇数
    pub fn size_to_clusters(&self, size: u32) -> u32 {
        (size + self.bytes_per_cluster - 1) / self.bytes_per_cluster
    }

    /// 计算当前偏移量在第几个簇
    pub fn cluster_of_offset(&self, offset: usize) -> u32 {
        //println!("cluster_of_offset: off = {}, bytes_per_cluster = {}",offset, self.bytes_per_cluster);
        offset as u32 / self.bytes_per_cluster
    }

    pub fn free_clusters(&self) -> u32 {
        self.fsinfo.read_free_clusters(self.block_device.clone())
    }

    /// 将长文件名拆分，并且补全0
    // DEBUG
    pub fn long_name_split(&self, name: &str) -> Vec<String> {
        let len = name.len() as u32; // 要有\0
        let name_bytes = name.as_bytes();
        let mut name_vec: Vec<String> = Vec::new();
        // 计算需要几个目录项，向上取整
        // 以13个字符为单位进行切割，每一组占据一个目录项
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

    /* 拆分文件名和后缀 */
    pub fn split_name_ext<'a>(&self, name: &'a str) -> (&'a str, &'a str) {
        let mut name_and_ext: Vec<&str> = name.split(".").collect(); // 按 . 进行分割
        if name_and_ext.len() == 1 {
            // 如果没有后缀名则推入一个空值
            name_and_ext.push("");
        }
        (name_and_ext[0], name_and_ext[1])
    }

    /* 将短文件名格式化为目录项存储的内容 */
    pub fn short_name_format(&self, name: &str) -> ([u8; 8], [u8; 3]) {
        let (mut name_, mut ext_) = self.split_name_ext(name);
        // 对这两个目录进行特殊处理（因为不能被正确分割）
        if name == "." || name == ".." {
            name_ = name;
            ext_ = "";
        }
        let name_bytes = name_.as_bytes();
        let ext_bytes = ext_.as_bytes();
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

    /* 由长文件名生成短文件名 */
    // DEBUG
    pub fn generate_short_name(&self, long_name: &str) -> String {
        // 目前仅支持【name.extension】 或者 【没有后缀】 形式的文件名！
        // 暂时不支持重复检测，即默认生成序号为~1
        // 无后缀
        let (name_, ext_) = self.split_name_ext(long_name);
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

    pub fn cache_write_back(&self) {
        write_to_dev();
    }
    // QUES 应该保留哪个checksum?
    /*
    pub fn checksum(short_name:[u8;11])->u8{
        let sum:u8 = 0;
        for i in 0..11{
            if (sum & 1) != 0 {
                sum = 0x80 + (sum>>1) + short_name[i];
            }else{
                sum = (sum>>1) + short_name[i];
            }
        }
        sum
    }*/
}
