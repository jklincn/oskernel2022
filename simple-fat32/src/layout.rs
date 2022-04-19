use super::{clone_into_array, fat32_manager::FAT32Manager, get_block_cache, get_info_cache, BlockDevice, CacheMode, BLOCK_SZ};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

const LEAD_SIGNATURE: u32 = 0x41615252; // This lead signature is used to validate that this is in fact an FSInfo sector.
const SECOND_SIGNATURE: u32 = 0x61417272; // Another signature that is more localized in the sector to the location of the fields that are used.
pub const FREE_CLUSTER: u32 = 0x00000000; // 空闲簇
pub const END_CLUSTER: u32 = 0x0FFFFFF8; // 最后一个簇
pub const BAD_CLUSTER: u32 = 0x0FFFFFF7;
const FATENTRY_PER_SEC: u32 = BLOCK_SZ as u32 / 4;

// 文件属性
pub const ATTR_READ_ONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_VOLUME_ID: u8 = 0x08;
pub const ATTR_DIRECTORY: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;
pub const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;
pub const ATTR_LONG_NAME_MASK: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID | ATTR_DIRECTORY | ATTR_ARCHIVE;

pub const DIRENT_SZ: usize = 32;
#[allow(unused)]
pub const SHORT_NAME_LEN: u32 = 8;
#[allow(unused)]
pub const SHORT_EXT_LEN: u32 = 3;
pub const LONG_NAME_LEN: u32 = 13;

pub const ALL_UPPER_CASE: u8 = 0x00;
pub const ALL_LOWER_CASE: u8 = 0x08;

type DataBlock = [u8; BLOCK_SZ];

/// DBR(Dos Boot Record) and BPB Structure
/// or call it BS(Boot sector)
#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct FatBS {
    #[allow(unused)]
    pub bs_jmp_boot: [u8; 3], // 跳转指令，指向启动代码
    #[allow(unused)]
    pub bs_oem_name: [u8; 8], // 建议值为“MSWIN4.1”
    pub bpb_bytes_per_sec: u16, // 每扇区的字节数
    pub bpb_sec_per_clus: u8,   // 每簇的扇区数
    pub bpb_rsvd_sec_cnt: u16,  // 保留扇区的数目
    pub bpb_num_fats: u8,       // FAT数
    pub bpb_root_ent_cnt: u16,  // 对于FAT12和FAT16此域包含根目录中目录的个数（每项长度为32字节），对于FAT32，此项必须为0。
    pub bpb_tot_sec16: u16,     // 早期版本中16bit的总扇区，对于FAT32，此域必为0。
    pub bpb_media: u8,          // 媒体描述符
    pub bpb_fatsz16: u16,       // FAT12/FAT16一个FAT表所占的扇区数，对于FAT32来说此域必须为0
    pub bpb_sec_per_trk: u16,   // 每磁道的扇区数，用于BIOS中断0x13
    pub bpb_num_heads: u16,     // 磁头数，用于BIOS的0x13中断
    pub bpb_hidd_sec: u32, // 在此FAT分区之前所隐藏的扇区数，必须使得调用BIOS的0x13中断可以得到此数值，对于那些没有分区的存储介质，此域必须为0
    pub bpb_tot_sec32: u32, // 该卷总扇区数（32bit），这里的扇区总数包括FAT卷四个个基本分的全部扇区，此域可以为0，若此域为0，BPB_ToSec16必须为非0，对FAT32，此域必须是非0。
}

impl FatBS {
    pub fn total_sectors(&self) -> u32 {
        if self.bpb_tot_sec16 == 0 {
            self.bpb_tot_sec32
        } else {
            self.bpb_tot_sec16 as u32
        }
    }

    /*第一个FAT表所在的扇区*/
    pub fn first_fat_sector(&self) -> u32 {
        self.bpb_rsvd_sec_cnt as u32
    }
}

/// FAT32 Structure Starting at Offset 36(0x24)
#[repr(packed)]
#[derive(Clone, Copy)]
#[allow(unused)]
pub struct FatExtBS {
    pub bpb_fatsz32: u32,          // 一个FAT表所占的扇区数，此域为FAT32特有，同时BPB_FATSz16必须为0
    pub bpb_ext_flags: u16,        // 扩展标志，此域FAT32特有
    pub bpb_fs_ver: u16,           // 此域为FAT32特有， 高位为FAT32的主版本号，低位为次版本号
    pub bpb_root_clus: u32,        // 	根目录所在第一个簇的簇号，通常该数值为2，但不是必须为2。
    pub bpb_fsinfo: u16,           // 保留区中FAT32卷FSINFO结构所占的扇区数，通常为1。
    pub bpb_bk_boot_sec: u16,      // 如果不为0，表示在保留区中引导记录的备数据所占的扇区数，通常为6。
    pub bpb_reserved: [u8; 12],    // 用于以后FAT扩展使用，对FAT32。此域用0填充
    pub bs_drv_num: u8,            // 用于BIOS中断0x13得到磁盘驱动器参数
    pub bs_reserved1: u8,          // 保留（供NT使用），格式化FAT卷时必须设为0
    pub bs_boot_sig: u8,           // 扩展引导标记（0x29）用于指明此后的3个域可用
    pub bs_vol_id: u32,            // 卷标序列号，此域以BS_VolLab一起可以用来检测磁盘是否正确
    pub bs_vol_lab: [u8; 11],      // 磁盘卷标，此域必须与根目录中11字节长的卷标一致。
    pub bs_fil_sys_type: [u8; 64], // 以下的几种之一：“FAT12”，“FAT16”，“FAT32”。
}

impl FatExtBS {
    // FAT占用的扇区数
    pub fn fat_size(&self) -> u32 {
        self.bpb_fatsz32
    }

    pub fn fat_info_sec(&self) -> u32 {
        self.bpb_fsinfo as u32
    }

    #[allow(unused)]
    pub fn root_clusters(&self) -> u32 {
        self.bpb_root_clus
    }
}

/*
FSInfo 字段
FSI_LeadSig	    0	4    Value 0x41615252
FSI_Reserved1	4	480 保留
FSI_StrucSig	484	4    Value 0x61417272
FSI_Free_Count	488	4   包含卷上最近已知的空闲簇计数。如果值是0xFFFFFFFF，那么空闲计数是未知的，必须计算。
FSI_Nxt_Free	492	4   通常，这个值设置为驱动程序分配的最后一个簇号
FSI_Reserved2	496	12  保留
FSI_TrailSig	508	4   Trail signature (0xAA550000)
 */
// 该结构体不对Buffer作结构映射，仅保留位置信息
// 但是为其中信息的获取和修改提供了接口
pub struct FSInfo {
    sector_num: u32,
}

impl FSInfo {
    pub fn new(sector_num: u32) -> Self {
        Self { sector_num }
    }

    fn check_lead_signature(&self, block_device: Arc<dyn BlockDevice>) -> bool {
        get_info_cache(self.sector_num as usize, block_device, CacheMode::READ)
            .read()
            .read(0, |&lead_sig: &u32| lead_sig == LEAD_SIGNATURE)
    }

    fn check_another_signature(&self, block_device: Arc<dyn BlockDevice>) -> bool {
        get_info_cache(self.sector_num as usize, block_device, CacheMode::READ)
            .read()
            .read(484, |&sec_sig: &u32| sec_sig == SECOND_SIGNATURE)
    }

    /*对签名进行校验*/
    pub fn check_signature(&self, block_device: Arc<dyn BlockDevice>) -> bool {
        return self.check_lead_signature(block_device.clone()) && self.check_another_signature(block_device.clone());
    }

    /*读取空闲簇数*/
    pub fn read_free_clusters(&self, block_device: Arc<dyn BlockDevice>) -> u32 {
        get_info_cache(self.sector_num as usize, block_device, CacheMode::READ)
            .read()
            .read(488, |&free_cluster_count: &u32| free_cluster_count)
    }

    /*写入空闲块数*/
    pub fn write_free_clusters(&self, free_clusters: u32, block_device: Arc<dyn BlockDevice>) {
        get_info_cache(self.sector_num as usize, block_device, CacheMode::WRITE)
            .write()
            .modify(488, |free_cluster_count: &mut u32| {
                *free_cluster_count = free_clusters;
            });
    }

    /*读取起始空闲块*/
    pub fn first_free_cluster(&self, block_device: Arc<dyn BlockDevice>) -> u32 {
        get_info_cache(self.sector_num as usize, block_device, CacheMode::READ)
            .read()
            .read(492, |&start_cluster: &u32| start_cluster)
    }

    /*写入起始空闲块*/
    pub fn write_first_free_cluster(&self, start_cluster: u32, block_device: Arc<dyn BlockDevice>) {
        //println!("sector_num = {}, start_c = {}", self.sector_num, start_cluster);
        get_info_cache(self.sector_num as usize, block_device, CacheMode::WRITE)
            .write()
            .modify(492, |start_clu: &mut u32| {
                *start_clu = start_cluster;
            });
    }
}

// FAT 32 Byte Directory Entry Structure
// 11+1+1+1+2+2+2+2+2+2+2+4 = 32
#[derive(Clone, Copy, Debug)]
#[repr(packed)]
#[allow(unused)]
pub struct ShortDirEntry {
    dir_name: [u8; 11],     // 短文件名
    dir_attr: u8,           // 文件属性
    dir_ntres: u8,          // Reserved for use by Windows NT
    dir_crt_time_tenth: u8, // Millisecond stamp at file creation time
    dir_crt_time: u16,      // Time file was created
    dir_crt_date: u16,      // Date file was created
    dir_lst_acc_date: u16,  // Last access date
    dir_fst_clus_hi: u16,   // High word of this entry’s first cluster number (always 0 for a FAT12 or FAT16 volume).
    dir_wrt_time: u16,      // Time of last write
    dir_wrt_date: u16,      // Date of last write
    dir_fst_clus_lo: u16,   // Low word of this entry’s first cluster number
    dir_file_size: u32,     // 32-bit DWORD holding this file’s size in bytes
}

impl ShortDirEntry {
    /* 建一个空的，一般读取时用到 */
    // QUES 真的用得到？
    pub fn empty() -> Self {
        Self {
            dir_name: [0; 11],
            dir_attr: 0,
            dir_ntres: 0,
            dir_crt_time_tenth: 0,
            dir_crt_time: 0,
            dir_crt_date: 0,
            dir_lst_acc_date: 0,
            dir_fst_clus_hi: 0,
            dir_wrt_time: 0,
            dir_wrt_date: 0,
            dir_fst_clus_lo: 0,
            dir_file_size: 0,
        }
    }

    /* 创建文件时调用
     * 新建时不必分配块。写时检测初始簇是否为0，为0则需要分配。
     */
    pub fn new(name_: &[u8], dir_attr: u8) -> Self {
        let dir_name: [u8; 11] = clone_into_array(&name_[0..11]);
        Self {
            dir_name,
            dir_attr,
            dir_ntres: 0,
            dir_crt_time_tenth: 0,
            dir_crt_time: 0,
            dir_crt_date: 0,
            dir_lst_acc_date: 0,
            dir_fst_clus_hi: 0,
            dir_wrt_time: 0,
            dir_wrt_date: 0,
            dir_fst_clus_lo: 0,
            dir_file_size: 0,
        }
    }

    pub fn initialize(&mut self, name_: &[u8], dir_attr: u8) {
        let dir_name: [u8; 11] = clone_into_array(&name_[0..11]);
        *self = Self {
            dir_name,
            dir_attr,
            dir_ntres: 0,
            dir_crt_time_tenth: 0,
            dir_crt_time: 0,
            dir_crt_date: 0,
            dir_lst_acc_date: 0,
            dir_fst_clus_hi: 0,
            dir_wrt_time: 0,
            dir_wrt_date: 0,
            dir_fst_clus_lo: 0,
            dir_file_size: 0,
        };
    }

    /* 返回目前使用的簇的数量 */
    pub fn data_clusters(&self, bytes_per_cluster: u32) -> u32 {
        // size为0的时候就是0
        (self.dir_file_size + bytes_per_cluster - 1) / bytes_per_cluster
    }

    /// If DIR_Name[0] == 0xE5, then the directory entry is free (there is no file or directory name in this entry).
    pub fn is_deleted(&self) -> bool {
        if self.dir_name[0] == 0xE5 {
            true
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.is_deleted() // 未删除即有效
    }

    /// If DIR_Name[0] == 0x00, then the directory entry is free (same as for 0xE5),
    /// and there are no allocated directory entries after this one (all of the DIR_Name[0]
    /// bytes in all of the entries after this one are also set to 0).The special 0 value, 0
    /// rather than the 0xE5 value, indicates to FAT file system driver code that the rest of
    /// the entries in this directory do not need to be examined because they are all free.
    pub fn is_empty(&self) -> bool {
        if self.dir_name[0] == 0x00 {
            true
        } else {
            false
        }
    }

    pub fn is_dir(&self) -> bool {
        if 0 != (self.dir_attr & ATTR_DIRECTORY) {
            true
        } else {
            false
        }
    }

    pub fn is_file(&self) -> bool {
        !self.is_dir() // 要么目录要么文件
    }

    pub fn is_long(&self) -> bool {
        if self.dir_attr == ATTR_LONG_NAME {
            true
        } else {
            false
        }
    }

    pub fn ldir_attr(&self) -> u8 {
        self.dir_attr
    }

    // pub fn get_creation_time(&self) -> (u32, u32, u32, u32, u32, u32, u64) {
    //     // year-month-day-Hour-min-sec-long_sec
    //     let year: u32 = ((self.dir_crt_date & 0xFE00) >> 9) as u32 + 1980;
    //     let month: u32 = ((self.dir_crt_date & 0x01E0) >> 5) as u32;
    //     let day: u32 = (self.dir_crt_date & 0x001F) as u32;
    //     let hour: u32 = ((self.dir_crt_time & 0xF800) >> 11) as u32;
    //     let min: u32 = ((self.dir_crt_time & 0x07E0) >> 5) as u32;
    //     let sec: u32 = ((self.dir_crt_time & 0x001F) << 1) as u32; // 秒数需要*2
    //     let long_sec: u64 =
    //         ((((year - 1970) * 365 + month * 30 + day) * 24 + hour) * 3600 + min * 60 + sec) as u64;
    //     (year, month, day, hour, min, sec, long_sec)
    // }

    // pub fn get_modification_time(&self) -> (u32, u32, u32, u32, u32, u32, u64) {
    //     // year-month-day-Hour-min-sec
    //     let year: u32 = ((self.dir_wrt_date & 0xFE00) >> 9) as u32 + 1980;
    //     let month: u32 = ((self.dir_wrt_date & 0x01E0) >> 5) as u32;
    //     let day: u32 = (self.dir_wrt_date & 0x001F) as u32;
    //     let hour: u32 = ((self.dir_wrt_time & 0xF800) >> 11) as u32;
    //     let min: u32 = ((self.dir_wrt_time & 0x07E0) >> 5) as u32;
    //     let sec: u32 = ((self.dir_wrt_time & 0x001F) << 1) as u32; // 秒数需要*2
    //     let long_sec: u64 =
    //         ((((year - 1970) * 365 + month * 30 + day) * 24 + hour) * 3600 + min * 60 + sec) as u64;
    //     (year, month, day, hour, min, sec, long_sec)
    // }

    // pub fn get_accessed_time(&self) -> (u32, u32, u32, u32, u32, u32, u64) {
    //     // year-month-day-Hour-min-sec
    //     let year: u32 = ((self.dir_lst_acc_date & 0xFE00) >> 9) as u32 + 1980;
    //     let month: u32 = ((self.dir_lst_acc_date & 0x01E0) >> 5) as u32;
    //     let day: u32 = (self.dir_lst_acc_date & 0x001F) as u32;
    //     let hour: u32 = 0;
    //     let min: u32 = 0;
    //     let sec: u32 = 0; // 没有相关信息，默认0
    //     let long_sec: u64 =
    //         ((((year - 1970) * 365 + month * 30 + day) * 24 + hour) * 3600 + min * 60 + sec) as u64;
    //     (year, month, day, hour, min, sec, long_sec)
    // }

    /// 获取文件起始簇号
    pub fn first_cluster(&self) -> u32 {
        ((self.dir_fst_clus_hi as u32) << 16) + (self.dir_fst_clus_lo as u32)
    }

    /// 获取短文件名
    pub fn get_name_uppercase(&self) -> String {
        let mut name: String = String::new();
        for i in 0..11 {
            name.push(self.dir_name[i] as char);
        }
        name
        // 检查合法性 todo
        // if self.dir_name[0] == 0x20 {
        //     panic!("get_name_uppercase_panic");
        // } else {
        //     for i in 0..11 {
        //         name.push(self.dir_name[i] as char);
        //     }
        //     name
        // }
    }

    pub fn get_name_lowercase(&self) -> String {
        let mut name: String = String::new();
        for i in 0..11 {
            name.push((self.dir_name[i] as char).to_ascii_lowercase());
        }
        name
    }
    
    /* 设置当前文件的大小 */
    // 簇的分配和回收实际要对FAT表操作
    pub fn set_size(&mut self, dir_file_size: u32) {
        self.dir_file_size = dir_file_size;
    }

    pub fn get_size(&self) -> u32 {
        self.dir_file_size
    }

    pub fn set_case(&mut self, case: u8) {
        self.dir_ntres = case;
    }

    /* 设置文件起始簇 */
    pub fn set_first_cluster(&mut self, cluster: u32) {
        self.dir_fst_clus_hi = ((cluster & 0xFFFF0000) >> 16) as u16; // 设置高位
        self.dir_fst_clus_lo = (cluster & 0x0000FFFF) as u16; // 设置低位
    }

    /* 清空文件，删除时使用 */
    pub fn clear(&mut self) {
        self.dir_file_size = 0;
        //self.name[0] = 0xE5;
        self.set_first_cluster(0);
    }

    pub fn delete(&mut self) {
        self.dir_file_size = 0;
        self.dir_name[0] = 0xE5;
        self.set_first_cluster(0);
        //TODO:回收cluster?
    }

    /// 获取文件偏移量所在的簇、扇区和偏移
    pub fn get_pos(
        &self,
        offset: usize,
        manager: &Arc<RwLock<FAT32Manager>>,
        fat: &Arc<RwLock<FAT>>,
        block_device: &Arc<dyn BlockDevice>,
    ) -> (u32, usize, usize) {
        let manager_reader = manager.read();
        let fat_reader = fat.read();
        let bytes_per_sector = manager_reader.bytes_per_sector() as usize;
        let bytes_per_cluster = manager_reader.bytes_per_cluster() as usize;
        let cluster_index = manager_reader.cluster_of_offset(offset);
        let current_cluster = fat_reader.get_cluster_at(self.first_cluster(), cluster_index, Arc::clone(block_device));
        let current_sector = manager_reader.first_sector_of_cluster(current_cluster)
            + (offset - cluster_index as usize * bytes_per_cluster) / bytes_per_sector;
        (current_cluster, current_sector, offset % bytes_per_sector)
    }

    /* 以偏移量读取文件，这里会对fat和manager加读锁 */
    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        manager: &Arc<RwLock<FAT32Manager>>,
        fat: &Arc<RwLock<FAT>>,
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        // 获取共享锁
        let manager_reader = manager.read();
        let fat_reader = fat.read();
        let bytes_per_sector = manager_reader.bytes_per_sector() as usize;
        let bytes_per_cluster = manager_reader.bytes_per_cluster() as usize;
        let mut current_off = offset;

        // 1、检查边界条件
        let end: usize;
        if self.is_dir() {
            let dir_file_size =
                bytes_per_cluster * fat_reader.count_claster_num(self.first_cluster() as u32, block_device.clone()) as usize;
            end = offset + buf.len().min(dir_file_size);
        } else {
            // 此次文件读取的最大范围，要么是偏移量位置加上缓冲区大小，要么是文件总大小
            end = (offset + buf.len()).min(self.dir_file_size as usize);
        }

        // 读取的内容是否超出了文件的范围
        if current_off >= end {
            return 0;
        }

        // 2、计算开始读取的位置
        let (c_clu, c_sec, _) = self.get_pos(offset, manager, &manager_reader.get_fat(), block_device);
        if c_clu >= END_CLUSTER {
            return 0;
        };
        let mut current_cluster = c_clu;
        let mut current_sector = c_sec;

        let mut read_size = 0usize;

        // 3、开始读取内容
        loop {
            // 将偏移量向上对齐扇区大小（一般是512
            let mut end_current_block = (current_off / bytes_per_sector + 1) * bytes_per_sector;
            // 计算当前块的结束位置
            end_current_block = end_current_block.min(end);
            let block_read_size = end_current_block - current_off;
            let dst = &mut buf[read_size..read_size + block_read_size];
            if self.is_dir() {
                get_info_cache(
                    // 目录项通过Infocache访问
                    current_sector,
                    Arc::clone(block_device),
                    CacheMode::READ,
                )
                .read()
                .read(0, |data_block: &DataBlock| {
                    let src = &data_block[current_off % BLOCK_SZ..current_off % BLOCK_SZ + block_read_size];
                    dst.copy_from_slice(src);
                });
            } else {
                get_block_cache(current_sector, Arc::clone(block_device), CacheMode::READ)
                    .read()
                    .read(0, |data_block: &DataBlock| {
                        let src = &data_block[current_off % BLOCK_SZ..current_off % BLOCK_SZ + block_read_size];
                        dst.copy_from_slice(src);
                    });
            }
            // 更新读取长度
            read_size += block_read_size;
            if end_current_block == end {
                break;
            }
            // 更新索引参数
            current_off = end_current_block;
            if current_off % bytes_per_cluster == 0 {
                current_cluster = fat_reader.get_next_cluster(current_cluster, Arc::clone(block_device));
                if current_cluster >= END_CLUSTER {
                    break;
                }
                current_sector = manager_reader.first_sector_of_cluster(current_cluster);
            } else {
                current_sector += 1; //没读完一个簇，直接进入下一扇区
            }
        }
        read_size
    }

    /* 以偏移量写文件，这里会对fat和manager加读锁 */
    pub fn write_at(
        &self,
        offset: usize,
        buf: &[u8],
        manager: &Arc<RwLock<FAT32Manager>>,
        fat: &Arc<RwLock<FAT>>,
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        // 获取共享锁
        let manager_reader = manager.read();
        let fat_reader = fat.read();
        let bytes_per_sector = manager_reader.bytes_per_sector() as usize;
        let bytes_per_cluster = manager_reader.bytes_per_cluster() as usize;
        let mut current_off = offset;
        let end: usize;
        if self.is_dir() {
            let dir_file_size = bytes_per_cluster * fat_reader.count_claster_num(self.first_cluster() as u32, block_device.clone()) as usize;
            end = offset + buf.len().min(dir_file_size); // DEBUG:约束上界
        } else {
            end = (offset + buf.len()).min(self.dir_file_size as usize);
        }

        let (c_clu, c_sec, _) = self.get_pos(offset, manager, &manager_reader.get_fat(), block_device);
        let mut current_cluster = c_clu;
        let mut current_sector = c_sec;
        let mut write_size = 0usize;

        loop {
            // 将偏移量向上对齐扇区大小（一般是512
            let mut end_current_block = (current_off / bytes_per_sector + 1) * bytes_per_sector;
            end_current_block = end_current_block.min(end);

            // 写
            let block_write_size = end_current_block - current_off;
            //println!("write cache: current_sector = {}", current_sector);
            if self.is_dir() {
                get_info_cache(
                    // 目录项通过infocache访问
                    current_sector,
                    Arc::clone(block_device),
                    CacheMode::READ,
                )
                .write()
                .modify(0, |data_block: &mut DataBlock| {
                    let src = &buf[write_size..write_size + block_write_size];
                    let dst = &mut data_block[current_off % BLOCK_SZ..current_off % BLOCK_SZ + block_write_size];
                    dst.copy_from_slice(src);
                });
            } else {
                get_block_cache(current_sector, Arc::clone(block_device), CacheMode::READ)
                    .write()
                    .modify(0, |data_block: &mut DataBlock| {
                        let src = &buf[write_size..write_size + block_write_size];
                        let dst = &mut data_block[current_off % BLOCK_SZ..current_off % BLOCK_SZ + block_write_size];
                        dst.copy_from_slice(src);
                    });
            }
            // 更新读取长度
            write_size += block_write_size;
            if end_current_block == end {
                break;
            }
            // 更新索引参数
            current_off = end_current_block;
            if current_off % bytes_per_cluster == 0 {
                current_cluster = fat_reader.get_next_cluster(current_cluster, Arc::clone(block_device));
                if current_cluster >= END_CLUSTER {
                    panic!("END_CLUSTER");
                }
                current_sector = manager_reader.first_sector_of_cluster(current_cluster);

            } else {
                current_sector += 1; //没读完一个簇，直接进入下一扇区
            }
        }
        write_size
    }

    pub fn checksum(&self)->u8{
        let mut name_buff:[u8;11] = [0u8;11]; 
        let mut sum:u8 = 0;
        for i in 0..11 { name_buff[i] = self.dir_name[i]; }
        for i in 0..11{ 
            if (sum & 1) != 0 {
                sum = 0x80 + (sum>>1) + name_buff[i];
            }else{
                sum = (sum>>1) + name_buff[i];
            }
        }
        sum
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENT_SZ) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENT_SZ) }
    }
}

// FAT Long Directory Entry Structure
// 1+10+1+1+1+12+2+4 = 32
#[repr(packed)]
#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct LongDirEntry {
    // use Unicode !!!
    // 如果是该文件的最后一个长文件名目录项，
    // 则将该目录项的序号与 0x40 进行“或（OR）运算”的结果写入该位置。
    // 长文件名要有\0
    ldir_ord: u8, // The order of this entry in the sequence of long dir entries associated with the short dir entry at the end of the long dir set.
    ldir_name1: [u8; 10], // 5characters
    ldir_attr: u8, // Attributes - must be ATTR_LONG_NAME
    ldir_type: u8, // If zero, indicates a directory entry that is a sub-component of a long name.Non-zero implies other dirent types.
    ldir_chksum: u8, // Checksum of name in the short dir entry at the end of the long dir set.
    ldir_name2: [u8; 12], // 6characters
    ldir_fst_clus_lo: [u8; 2], // Must be ZERO
    ldir_name3: [u8; 4], // 2characters
}

impl From<&[u8]> for LongDirEntry {
    fn from(bytes: &[u8]) -> Self {
        Self {
            ldir_ord: bytes[0],
            ldir_name1: clone_into_array(&bytes[1..11]),
            ldir_attr: bytes[11],
            ldir_type: bytes[12],
            ldir_chksum: bytes[13],
            ldir_name2: clone_into_array(&bytes[14..26]),
            ldir_fst_clus_lo: clone_into_array(&bytes[26..28]),
            ldir_name3: clone_into_array(&bytes[28..32]),
        }
    }
}

impl LongDirEntry {
    pub fn empty() -> Self {
        Self {
            ldir_ord: 0,
            ldir_name1: [0; 10],
            ldir_attr: 0,
            ldir_type: 0,
            ldir_chksum: 0,
            ldir_name2: [0; 12],
            ldir_fst_clus_lo: [0; 2],
            ldir_name3: [0; 4],
        }
    }

    pub fn ldir_attr(&self) -> u8 {
        self.ldir_attr
    }

    pub fn is_empty(&self) -> bool {
        if self.ldir_ord == 0x00 {
            true
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.ldir_ord == 0xE5 {
            false
        } else {
            true
        }
    }

    pub fn is_deleted(&self) -> bool {
        !self.is_valid()
    }

    /* 上层要完成对namebuffer的填充，注意\0，以及checksum的计算 */
    /* 目前只支持英文，因此传入ascii */
    pub fn initialize(&mut self, name_buffer: &[u8], ldir_ord: u8, ldir_chksum: u8) {
        let ord = ldir_ord;
        //println!("** initialize namebuffer = {:?}", name_buffer);
        //if is_last { ord = ord | 0x40 }
        let mut ldir_name1: [u8; 10] = [0; 10];
        let mut ldir_name2: [u8; 12] = [0; 12];
        let mut ldir_name3: [u8; 4] = [0; 4];
        let mut end_offset = 0;
        for i in 0..5 {
            if end_offset == 0 {
                ldir_name1[i << 1] = name_buffer[i];
                if name_buffer[i] == 0 {
                    end_offset = i;
                }
            } else {
                ldir_name1[i << 1] = 0xFF;
                ldir_name1[(i << 1) + 1] = 0xFF;
            }
        }
        for i in 5..11 {
            if end_offset == 0 {
                ldir_name2[(i - 5) << 1] = name_buffer[i];
                if name_buffer[i] == 0 {
                    end_offset = i;
                }
            } else {
                ldir_name2[(i - 5) << 1] = 0xFF;
                ldir_name2[((i - 5) << 1) + 1] = 0xFF;
            }
        }
        for i in 11..13 {
            if end_offset == 0 {
                ldir_name3[(i - 11) << 1] = name_buffer[i];
                if name_buffer[i] == 0 {
                    end_offset = i;
                }
            } else {
                ldir_name3[(i - 11) << 1] = 0xFF;
                ldir_name3[((i - 11) << 1) + 1] = 0xFF;
            }
        }
        *self = Self {
            ldir_ord: ord,
            ldir_name1,
            ldir_attr: ATTR_LONG_NAME,
            ldir_type: 0,
            ldir_chksum,
            ldir_name2,
            ldir_fst_clus_lo: [0u8; 2],
            ldir_name3,
        }
    }

    pub fn clear(&mut self) {
        //self.LDIR_Ord = 0xE5;
    }

    pub fn delete(&mut self) {
        self.ldir_ord = 0xE5;
    }

    /* 获取长文件名，此处完成unicode至ascii的转换，暂不支持中文！*/
    // 这里直接将几个字段拼合，不考虑填充字符0xFF等
    // 需要和manager的long_name_split配合使用
    pub fn get_name_raw(&self) -> String {
        let mut name = String::new();
        let mut c: u8;
        for i in 0..5 {
            c = self.ldir_name1[i << 1];
            //if c == 0 { return name }
            name.push(c as char);
        }
        for i in 0..6 {
            c = self.ldir_name2[i << 1];
            //if c == 0 { return name }
            name.push(c as char);
        }
        for i in 0..2 {
            c = self.ldir_name3[i << 1];
            //if c == 0 { return name }
            name.push(c as char);
        }
        return name;
    }

    pub fn get_name_format(&self) -> String {
        let mut name = String::new();
        let mut c: u8;
        for i in 0..5 {
            c = self.ldir_name1[i << 1];
            if c == 0 {
                return name;
            }
            name.push(c as char);
        }
        for i in 0..6 {
            c = self.ldir_name2[i << 1];
            if c == 0 {
                return name;
            }
            name.push(c as char);
        }
        for i in 0..2 {
            c = self.ldir_name3[i << 1];
            if c == 0 {
                return name;
            }
            name.push(c as char);
        }
        return name;
    }

    #[allow(unused)]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENT_SZ) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENT_SZ) }
    }
    pub fn get_order(&self) -> u8 {
        self.ldir_ord
    }
    pub fn get_checksum(&self) -> u8 {
        self.ldir_chksum
    }
}

// 常驻内存，不作一一映射
#[allow(unused)]
#[derive(Clone, Copy)]
pub struct FAT {
    fat1_sector: u32, // FAT1的起始扇区
    fat2_sector: u32, // FAT2的起始扇区
    n_sectors: u32,   // FAT表大小（扇区数量）
    n_entry: u32,     //表项数量
}

// TODO: 防越界处理（虽然可能这辈子都遇不到）
impl FAT {
    pub fn new(fat1_sector: u32, fat2_sector: u32, n_sectors: u32, n_entry: u32) -> Self {
        Self {
            fat1_sector,
            fat2_sector,
            n_sectors,
            n_entry,
        }
    }

    /// 计算簇号对应表项所在的扇区和偏移
    fn calculate_pos(&self, cluster: u32) -> (u32, u32, u32) {
        // 返回sector号和offset
        // 前为FAT1的扇区号，后为FAT2的扇区号，最后为offset
        // DEBUG
        let fat1_sec = self.fat1_sector + cluster / FATENTRY_PER_SEC;
        let fat2_sec = self.fat2_sector + cluster / FATENTRY_PER_SEC;
        let offset = 4 * (cluster % FATENTRY_PER_SEC);
        (fat1_sec, fat2_sec, offset)
    }

    /// 获取可用簇的簇号
    pub fn next_free_cluster(&self, current_cluster: u32, block_device: Arc<dyn BlockDevice>) -> u32 {
        // DEBUG
        let mut curr_cluster = current_cluster + 1;
        loop {
            #[allow(unused)]
            let (fat1_sec, fat2_sec, offset) = self.calculate_pos(curr_cluster);
            // 查看当前cluster的表项
            let entry_val = get_info_cache(fat1_sec as usize, block_device.clone(), CacheMode::READ)
                .read()
                .read(offset as usize, |&entry_val: &u32| entry_val);
            if entry_val == FREE_CLUSTER {
                break;
            } else {
                curr_cluster += 1;
            }
        }
        curr_cluster & 0x0FFFFFFF
    }

    /// 查询当前簇的下一个簇
    pub fn get_next_cluster(&self, cluster: u32, block_device: Arc<dyn BlockDevice>) -> u32 {
        // 需要对损坏簇作出判断
        // 及时使用备用表
        // 无效或未使用返回0
        let (fat1_sec, fat2_sec, offset) = self.calculate_pos(cluster);
        //println!("fat1_sec={} offset = {}", fat1_sec, offset);
        let fat1_rs = get_info_cache(fat1_sec as usize, block_device.clone(), CacheMode::READ)
            .read()
            .read(offset as usize, |&next_cluster: &u32| next_cluster);
        let fat2_rs = get_info_cache(fat2_sec as usize, block_device.clone(), CacheMode::READ)
            .read()
            .read(offset as usize, |&next_cluster: &u32| next_cluster);
        if fat1_rs == BAD_CLUSTER {
            if fat2_rs == BAD_CLUSTER {
                0
            } else {
                fat2_rs & 0x0FFFFFFF
            }
        } else {
            fat1_rs & 0x0FFFFFFF
        }
    }

    pub fn set_end(&self, cluster: u32, block_device: Arc<dyn BlockDevice>) {
        self.set_next_cluster(cluster, END_CLUSTER, block_device);
    }

    /// 设置当前簇的下一个簇
    pub fn set_next_cluster(&self, cluster: u32, next_cluster: u32, block_device: Arc<dyn BlockDevice>) {
        // 同步修改两个FAT
        // 注意设置末尾项为 0x0FFFFFF8
        //assert_ne!(next_cluster, 0);
        let (fat1_sec, fat2_sec, offset) = self.calculate_pos(cluster);
        get_info_cache(fat1_sec as usize, block_device.clone(), CacheMode::WRITE)
            .write()
            .modify(offset as usize, |old_clu: &mut u32| {
                *old_clu = next_cluster;
            });
        get_info_cache(fat2_sec as usize, block_device.clone(), CacheMode::WRITE)
            .write()
            .modify(offset as usize, |old_clu: &mut u32| {
                *old_clu = next_cluster;
            });
    }

    /// 获取某个簇链的第i个簇(i为参数)
    pub fn get_cluster_at(&self, start_cluster: u32, index: u32, block_device: Arc<dyn BlockDevice>) -> u32 {
        // 如果有异常，返回0
        //println!("** get_cluster_at index = {}",index);
        let mut cluster = start_cluster;
        #[allow(unused)]
        for i in 0..index {
            //print!("in fat curr cluster = {}", cluster);
            cluster = self.get_next_cluster(cluster, block_device.clone());
            //println!(", next cluster = {:X}", cluster);
            if cluster == 0 {
                break;
            }
        }
        cluster & 0x0FFFFFFF
    }

    /// 获取某个簇链的最后一个簇
    pub fn final_cluster(&self, start_cluster: u32, block_device: Arc<dyn BlockDevice>) -> u32 {
        let mut curr_cluster = start_cluster;
        assert_ne!(start_cluster, 0);
        loop {
            let next_cluster = self.get_next_cluster(curr_cluster, block_device.clone());
            //println!("in fianl cl {};{}", curr_cluster, next_cluster);
            //assert_ne!(next_cluster, 0);
            if next_cluster >= END_CLUSTER || next_cluster == 0 {
                return curr_cluster & 0x0FFFFFFF;
            } else {
                curr_cluster = next_cluster;
            }
        }
    }

    /// 获得某个簇链从指定簇开始的所有簇
    pub fn get_all_cluster_of(&self, start_cluster: u32, block_device: Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut curr_cluster = start_cluster;
        let mut v_cluster: Vec<u32> = Vec::new();
        loop {
            v_cluster.push(curr_cluster & 0x0FFFFFFF);
            let next_cluster = self.get_next_cluster(curr_cluster, block_device.clone());
            //println!("in all, curr = {}, next = {}", curr_cluster, next_cluster);
            //assert_ne!(next_cluster, 0);
            if next_cluster >= END_CLUSTER || next_cluster == 0 {
                return v_cluster;
            } else {
                curr_cluster = next_cluster;
            }
        }
    }

    /// 统计某个簇链从指定簇开始到结尾的簇数
    pub fn count_claster_num(&self, start_cluster: u32, block_device: Arc<dyn BlockDevice>) -> u32 {
        if start_cluster == 0 {
            return 0;
        }
        let mut curr_cluster = start_cluster;
        let mut count: u32 = 0;
        loop {
            count += 1;
            let next_cluster = self.get_next_cluster(curr_cluster, block_device.clone());
            //println!("next_cluster = {:X}",next_cluster);
            if next_cluster >= END_CLUSTER || next_cluster > 0xF000000 {
                return count;
            } else {
                curr_cluster = next_cluster;
            }
        }
    }
}
