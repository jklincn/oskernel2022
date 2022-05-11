
use super::{fat32_manager::*, get_info_cache, layout::*, BlockDevice};
use alloc::string::{self, String};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

#[derive(Clone)]
pub struct VFile {
    name: String,
    short_sector: usize,               // 文件短目录项所在扇区
    short_offset: usize,               // 文件短目录项所在扇区的偏移
    long_pos_vec: Vec<(usize, usize)>, // 长目录项的位置<sector, offset>
    attribute: u8,
    fs: Arc<RwLock<FAT32Manager>>,
    block_device: Arc<dyn BlockDevice>,
}

// QUES 文件的复制、移动、删除(尤其目录)的加锁方式需要考证。。。
// 理论上，操作的时候通过cache锁short_dirent就有很好的效果

// 以数组的形式读目录项 QUES:是否有更好的？
// type DirEntryArray = [ShortDirEntry; 16]; // 一般是16...

impl VFile {
    pub fn new(
        name: String,
        short_sector: usize,
        short_offset: usize,
        long_pos_vec: Vec<(usize, usize)>,
        attribute: u8,
        fs: Arc<RwLock<FAT32Manager>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            name,
            short_sector,
            short_offset,
            long_pos_vec,
            attribute,
            fs,
            block_device,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn file_size(&self) -> u32 {
        self.read_short_dirent(|se: &ShortDirEntry| se.file_size())
    }

    pub fn is_dir(&self) -> bool {
        if 0 != (self.attribute & ATTR_DIRECTORY) {
            true
        } else {
            false
        }
    }

    pub fn is_short(&self) -> bool {
        if self.long_pos_vec.len() == 0 {
            true
        } else {
            false
        }
    }

    fn read_short_dirent<V>(&self, f: impl FnOnce(&ShortDirEntry) -> V) -> V {
        if self.short_sector == 0 {
            // 根目录项
            let root_dirent = self.fs.read().get_root_dirent();
            let rr = root_dirent.read();
            f(&rr)
        } else {
            get_info_cache(self.short_sector, self.block_device.clone())
                .read()
                .read(self.short_offset, f)
        }
    }

    fn modify_long_dirent<V>(&self, index: usize, f: impl FnOnce(&mut LongDirEntry) -> V) -> V {
        let (sector, offset) = self.long_pos_vec[index];
        get_info_cache(sector, self.block_device.clone()).write().modify(offset, f)
    }

    fn modify_short_dirent<V>(&self, f: impl FnOnce(&mut ShortDirEntry) -> V) -> V {
        if self.short_sector == 0 {
            //println!("[fs]: modify vroot dent");
            let root_dirent = self.fs.read().get_root_dirent();
            let mut rw = root_dirent.write();
            f(&mut rw)
        } else {
            get_info_cache(self.short_sector, self.block_device.clone())
                .write()
                .modify(self.short_offset, f)
        }
    }

    /// 获取文件偏移量所在的扇区和偏移
    fn get_pos(&self, offset: usize) -> (usize, usize) {
        let (_, section, offset) = self.read_short_dirent(|short_entry: &ShortDirEntry| {
            short_entry.get_pos(offset, &self.fs, &self.fs.read().get_fat(), &self.block_device)
        });
        (section, offset)
    }

    fn first_cluster(&self) -> u32 {
        self.read_short_dirent(|se: &ShortDirEntry| se.first_cluster())
    }

    fn find_long_name(&self, name: &str, dir_ent: &ShortDirEntry) -> Option<VFile> {
        // 拆分长文件名
        let mut name_vec = long_name_split(name);
        let long_ent_num = name_vec.len();
        let mut offset: usize = 0;
        let mut long_entry = LongDirEntry::new();

        let mut long_pos_vec: Vec<(usize, usize)> = Vec::new();
        let name_last = name_vec.pop().unwrap();

        loop {
            long_pos_vec.clear();
            // 读取offset处的目录项
            let mut read_size = dir_ent.read_at(
                offset,
                long_entry.as_bytes_mut(),
                &self.fs,
                &self.fs.read().get_fat(),
                &self.block_device,
            );
            if read_size != DIRENT_SZ || long_entry.is_empty() {
                return None;
            }
            // 先匹配最后一个长文件名目录项，即长文件名的最后一块
            if long_entry.get_name_raw() == name_last && long_entry.attr() == ATTR_LONG_NAME {
                // 如果名称一致，则获取 order进行下一步校验
                let mut order = long_entry.order();
                // 校验 order的合法性，不合法则跳过继续搜索
                if order & 0x40 == 0 || order == 0xE5 {
                    offset += DIRENT_SZ;
                    continue;
                }
                // 恢复 order为正确的次序值
                order = order ^ 0x40;
                // 如果长文件名目录项数量对不上，则跳过继续搜索
                if order as usize != long_ent_num {
                    offset += DIRENT_SZ;
                    continue;
                }
                // 如果order匹配通过，开一个循环继续匹配长名目录项
                let mut is_match = true;
                for i in 1..order as usize {
                    read_size = dir_ent.read_at(
                        offset + i * DIRENT_SZ,
                        long_entry.as_bytes_mut(),
                        &self.fs,
                        &self.fs.read().get_fat(),
                        &self.block_device,
                    );
                    if read_size != DIRENT_SZ {
                        return None;
                    }
                    // 匹配前一个名字段，如果失败就退出
                    if long_entry.get_name_raw() != name_vec[long_ent_num - 1 - i] || long_entry.attr() != ATTR_LONG_NAME {
                        is_match = false;
                        break;
                    }
                }
                if is_match {
                    // 如果成功，读短目录项，进行校验
                    let checksum = long_entry.check_sum();
                    let mut short_entry = ShortDirEntry::new();
                    let short_entry_offset = offset + long_ent_num * DIRENT_SZ;
                    read_size = dir_ent.read_at(
                        short_entry_offset,
                        short_entry.as_bytes_mut(),
                        &self.fs,
                        &self.fs.read().get_fat(),
                        &self.block_device,
                    );
                    if read_size != DIRENT_SZ {
                        return None;
                    }
                    if short_entry.is_valid() && checksum == short_entry.checksum() {
                        let (short_sector, short_offset) = self.get_pos(short_entry_offset);
                        for i in 0..order as usize {
                            // 存入长名目录项位置了，第一个在栈顶
                            let pos = self.get_pos(offset + i * DIRENT_SZ);
                            long_pos_vec.push(pos);
                        }
                        return Some(VFile::new(
                            String::from(name),
                            short_sector,
                            short_offset,
                            long_pos_vec,
                            short_entry.attr(),
                            self.fs.clone(),
                            self.block_device.clone(),
                        ));
                    }
                }
            }
            offset += DIRENT_SZ;
        }
    }

    fn find_short_name(&self, name: &str, dir_ent: &ShortDirEntry) -> Option<VFile> {
        let name_upper = name.to_ascii_uppercase();
        let mut short_entry = ShortDirEntry::new();
        let mut offset = 0;
        let mut read_size: usize;
        loop {
            read_size = dir_ent.read_at(
                offset,
                short_entry.as_bytes_mut(),
                &self.fs,
                &self.fs.read().get_fat(),
                &self.block_device,
            );
            if read_size != DIRENT_SZ || short_entry.is_empty() {
                return None;
            } else {
                // 判断名字是否一样
                if short_entry.is_valid() && name_upper == short_entry.get_name_uppercase() {
                    let (short_sector, short_offset) = self.get_pos(offset);
                    let long_pos_vec: Vec<(usize, usize)> = Vec::new();
                    return Some(VFile::new(
                        String::from(name),
                        short_sector,
                        short_offset,
                        long_pos_vec,
                        short_entry.attr(),
                        self.fs.clone(),
                        self.block_device.clone(),
                    ));
                } else {
                    offset += DIRENT_SZ;
                    continue;
                }
            }
        }
    }

    /// 根据名称搜索当前目录下的文件
    pub fn find_vfile_byname(&self, name: &str) -> Option<VFile> {
        // 不是目录则退出
        assert!(self.is_dir());
        let (name_, ext_) = split_name_ext(name);
        // FAT32目录没有大小，只能搜，read_at已经做了完善的适配
        self.read_short_dirent(|short_entry: &ShortDirEntry| {
            if name_.len() > 8 || ext_.len() > 3 {
                //长文件名
                return self.find_long_name(name, short_entry);
            } else {
                // 短文件名
                return self.find_short_name(name, short_entry);
            }
        })
    }

    /// 根据路径递归搜索文件
    pub fn find_vfile_bypath(&self, path: Vec<&str>) -> Option<Arc<VFile>> {
        let _ = self.fs.read(); // 获取读锁
        let len = path.len();
        if len == 0 {
            return Some(Arc::new(self.clone()));
        }
        let mut current_vfile = self.clone();
        //println!("len:{}",len);
        for i in 0..len {
            if path[i] == "" || path[i] == "." {
                //println!("continue i={}",i);
                continue;
            }
            if let Some(vfile) = current_vfile.find_vfile_byname(path[i]) {
                current_vfile = vfile;
            } else {
                return None;
            }
        }
        //println!("return current_vfile.name={}",current_vfile.name());
        Some(Arc::new(current_vfile))
    }

    /* WAITING 既然目录都没有大小，那暂时没必要做这个 */
    //#[allow(unused)]
    //fn decrease_size() {}

    fn increase_size(&self, new_size: u32) {
        // TODO: return sth when cannot increase
        //println!("===================== in increase =======================");
        let first_cluster = self.first_cluster();
        let old_size = self.file_size();
        let manager_writer = self.fs.write();
        if new_size <= old_size {
            return;
        }
        let needed = manager_writer.cluster_num_needed(old_size, new_size, self.is_dir(), first_cluster);
        if needed == 0 {
            if !self.is_dir() {
                //self.size = new_size;
                self.modify_short_dirent(|se: &mut ShortDirEntry| {
                    se.set_file_size(new_size);
                });
            }
            return;
        }

        // println!("first cluster = {} nxt = {}", first_cluster, manager_writer.get_fat().read().get_next_cluster(first_cluster, self.block_device.clone()));
        if let Some(cluster) = manager_writer.alloc_cluster(needed) {
            // println!("*** cluster alloc = {}",cluster);
            if first_cluster == 0 {
                //未分配簇
                drop(manager_writer);
                self.modify_short_dirent(|se: &mut ShortDirEntry| {
                    se.set_first_cluster(cluster);
                });
            } else {
                // 已经分配簇
                //let fs_reader = self.fs.read();
                //println!("[fs-inc]: file: {}, newsz = {}", self.name(), new_size);
                //println!("  cluster alloc = {}",cluster);
                let fat = manager_writer.get_fat();
                //println!("try lock1");
                let fat_writer = fat.write();
                //println!("get lock1");
                let final_cluster = fat_writer.final_cluster(first_cluster, self.block_device.clone());
                assert_ne!(cluster, 0);
                fat_writer.set_next_cluster(final_cluster, cluster, self.block_device.clone());
                //let allc = fat_writer.get_all_cluster_of(first_cluster, self.block_device.clone());
                //println!("  finish set next cluster, cluster chain:{:?}", allc);
                drop(manager_writer);
            }
            self.modify_short_dirent(|se: &mut ShortDirEntry| {
                se.set_file_size(new_size);
            });
        } else {
            panic!("SD Card no space!!!");
        }
    }

    /*
    pub fn set_first_cluster(&mut self, first_cluster:u32) {
        self.first_cluster = first_cluster;
        self.modify_short_dirent(|se:&mut ShortDirEntry|{
            se.set_first_cluster(first_cluster);
        });
    }*/

    /* 在当前目录下创建文件 */
    pub fn create(&self, name: &str, attribute: u8) -> Option<Arc<VFile>> {
        // 检测同名文件
        assert!(self.is_dir());
        let manager_reader = self.fs.read();
        let (name_, ext_) = split_name_ext(name);
        // 搜索空处
        let mut dirent_offset: usize;
        if let Some(offset) = self.find_free_dirent() {
            dirent_offset = offset;
        } else {
            return None;
        }
        // 定义一个空的短文件名目录项用于写入
        let mut tmp_short_ent = ShortDirEntry::new();
        if name_.len() > 8 || ext_.len() > 3 {
            // 长文件名
            // 生成短文件名及对应目录项
            let short_name = generate_short_name(name);
            let (_name, _ext) = short_name_format(short_name.as_str());
            tmp_short_ent.initialize(&_name, &_ext, attribute);

            // 长文件名拆分
            let mut v_long_name = long_name_split(name);
            drop(manager_reader);
            let long_ent_num = v_long_name.len(); // 需要创建的长文件名目录项个数

            // 计算校验和
            let check_sum = tmp_short_ent.checksum();

            // 定义一个空的长文件名目录项用于写入
            let mut tmp_long_ent = LongDirEntry::new();
            // 逐个写入长名目录项
            for i in 0..long_ent_num {
                // 按倒序填充长文件名目录项，目的是为了避免名字混淆
                let mut order: u8 = (long_ent_num - i) as u8;
                if i == 0 {
                    // 最后一个长文件名目录项，将该目录项的序号与 0x40 进行或运算然后写入
                    order |= 0x40;
                }
                // 初始化长文件名目录项
                tmp_long_ent.initialize(v_long_name.pop().unwrap().as_bytes(), order, check_sum);
                // 写入长文件名目录项
                assert_eq!(self.write_at(dirent_offset, tmp_long_ent.as_bytes_mut()), DIRENT_SZ);
                // 更新写入位置
                dirent_offset += DIRENT_SZ;
            }
        } else {
            // 短文件名
            let (_name, _ext) = short_name_format(name);
            tmp_short_ent.initialize(&_name, &_ext, attribute);
            drop(manager_reader);
        }

        // 写短目录项（长文件名也是有短文件名目录项的）
        assert_eq!(self.write_at(dirent_offset, tmp_short_ent.as_bytes_mut()), DIRENT_SZ);
        // 这边的 if let 算是一个验证
        if let Some(vfile) = self.find_vfile_byname(name) {
            // 如果是目录类型，需要创建.和..
            if attribute & ATTR_DIRECTORY != 0 {
                let (_name, _ext) = short_name_format(".");
                let mut self_dir = ShortDirEntry::new();
                self_dir.initialize(&_name, &_ext, ATTR_DIRECTORY);
                self_dir.set_first_cluster(self.first_cluster());
                vfile.write_at(0, self_dir.as_bytes_mut());

                let (_name, _ext) = short_name_format("..");
                let mut par_dir = ShortDirEntry::new();
                par_dir.initialize(&_name, &_ext, ATTR_DIRECTORY);
                par_dir.set_first_cluster(self.first_cluster());
                vfile.write_at(DIRENT_SZ, par_dir.as_bytes_mut());

            }
            return Some(Arc::new(vfile));
        } else {
            None
        }
    }

    // /* 获取当前目录下的所有文件名以及属性，以Vector形式返回 */
    // // 如果出现错误，返回None
    // pub fn ls(&self) -> Option<Vec<(String, u8)>> {
    //     if !self.is_dir() {
    //         return None;
    //     }
    //     let mut list: Vec<(String, u8)> = Vec::new();
    //     // DEBUG
    //     let mut offset: usize = 0;
    //     let mut short_entry = ShortDirEntry::new();
    //     loop {
    //         let mut read_size = self.read_short_dirent(|curr_ent: &ShortDirEntry| {
    //             curr_ent.read_at(
    //                 offset,
    //                 short_entry.as_bytes_mut(),
    //                 &self.fs,
    //                 &self.fs.read().get_fat(),
    //                 &self.block_device,
    //             )
    //         });
    //         // 检测是否结束或被删除
    //         if read_size != DIRENT_SZ || short_entry.is_empty() {
    //             return Some(list);
    //         }
    //         if short_entry.is_deleted() {
    //             offset += DIRENT_SZ;
    //             continue;
    //         }
    //         if short_entry.is_long() {
    //             // 长文件名
    //             let (_, long_ent_list, _) = unsafe { short_entry.as_bytes_mut().align_to_mut::<LongDirEntry>() };
    //             // DEBUG
    //             let mut long_entry = long_ent_list[0];
    //             let mut order = long_entry.order(); //^ 0x40;
    //             if order & 0x40 == 0 {
    //                 offset += DIRENT_SZ;
    //                 continue;
    //             } else {
    //                 order = order ^ 0x40;
    //             }
    //             let mut name = long_entry.get_name_raw();
    //             #[allow(unused)]
    //             for i in 1..order as usize {
    //                 offset += DIRENT_SZ;
    //                 read_size = self.read_short_dirent(|curr_ent: &ShortDirEntry| {
    //                     curr_ent.read_at(
    //                         offset,
    //                         long_entry.as_bytes_mut(),
    //                         &self.fs,
    //                         &self.fs.read().get_fat(),
    //                         &self.block_device,
    //                     )
    //                 });
    //                 if read_size != DIRENT_SZ || long_entry.is_empty() || long_entry.is_deleted() {
    //                     return Some(list);
    //                 }
    //                 // 若无误，把该段名字放在name最前
    //                 name.insert_str(0, long_entry.get_name_raw().as_str());
    //             }
    //             // 从短文件获取类型
    //             offset += DIRENT_SZ;
    //             read_size = self.read_short_dirent(|curr_ent: &ShortDirEntry| {
    //                 curr_ent.read_at(
    //                     offset,
    //                     long_entry.as_bytes_mut(),
    //                     &self.fs,
    //                     &self.fs.read().get_fat(),
    //                     &self.block_device,
    //                 )
    //             });
    //             if read_size != DIRENT_SZ || long_entry.is_empty() || long_entry.is_deleted() {
    //                 return Some(list);
    //             }
    //             list.push((name, long_entry.attr()));
    //             offset += DIRENT_SZ;
    //             continue;
    //         } else {
    //             // 短文件名
    //             list.push((short_entry.get_name_lowercase(), short_entry.attr()));
    //             offset += DIRENT_SZ;
    //             continue;
    //         }
    //     }
    // }

    /* 获取目录中offset处目录项的信息 TODO:之后考虑和stat复用
     * 返回<name, offset, firstcluster,attributes>
     */
    pub fn dirent_info(&self, offset: usize) -> Option<(String, u32, u32, u8)> {
        if !self.is_dir() {
            return None;
        }
        let mut long_entry = LongDirEntry::new();
        let mut offset = offset;
        let mut name = String::new();
        let mut is_long = false;
        //let mut order:u8 = 0;
        loop {
            let read_size = self.read_short_dirent(|curr_ent: &ShortDirEntry| {
                curr_ent.read_at(
                    offset,
                    long_entry.as_bytes_mut(),
                    &self.fs,
                    &self.fs.read().get_fat(),
                    &self.block_device,
                )
            });
            if read_size != DIRENT_SZ || long_entry.is_empty() {
                return None;
            }
            if long_entry.is_deleted() {
                //if meet delete ent, search should be restart
                offset += DIRENT_SZ;
                name.clear();
                is_long = false;
                continue;
            }
            // 名称拼接
            if long_entry.attr() != ATTR_LONG_NAME {
                let (_, se_array, _) = unsafe { long_entry.as_bytes_mut().align_to_mut::<ShortDirEntry>() };
                let short_entry = se_array[0];
                if !is_long {
                    name = short_entry.get_name_lowercase();
                }
                //println!("---{}", short_entry.get_name_lowercase());
                let attribute = short_entry.attr();
                let first_cluster = short_entry.first_cluster();
                offset += DIRENT_SZ;
                return Some((name, offset as u32, first_cluster, attribute));
            } else {
                is_long = true;
                //order += 1;
                name.insert_str(0, long_entry.get_name_format().as_str());
            }
            offset += DIRENT_SZ;
        }
    }

    /* 获取目录中offset处目录项的信息 TODO:之后考虑和stat复用
     * 返回<size, atime, mtime, ctime>
     */
    // pub fn stat(&self) -> (i64, i64, i64, i64, u64) {
    //     self.read_short_dirent(|sde: &ShortDirEntry| {
    //         let (_, _, _, _, _, _, ctime) = sde.get_creation_time();
    //         let (_, _, _, _, _, _, atime) = sde.get_accessed_time();
    //         let (_, _, _, _, _, _, mtime) = sde.get_modification_time();
    //         let mut size = sde.file_size();
    //         let first_clu = sde.first_cluster();
    //         if self.is_dir() {
    //             let fs_reader = self.fs.read();
    //             let fat = fs_reader.get_fat();
    //             let fat_reader = fat.read();
    //             let cluster_num = fat_reader.count_claster_num(first_clu, self.block_device.clone());
    //             size = cluster_num * fs_reader.bytes_per_cluster();
    //             //println!("{} {}",cluster_num, fs_reader.bytes_per_cluster());
    //         }
    //         (size as i64, atime as i64, mtime as i64, ctime as i64, first_clu as u64)
    //     })
    // }

    // ls，返回二元组，第一个是文件名，第二个是文件属性（文件或者目录）
    pub fn ls(&self) -> Option<Vec<(String, u8)>> {
        if !self.is_dir() {
            return None;
        }
        let mut list: Vec<(String, u8)> = Vec::new();
        let mut file_entry = LongDirEntry::new();
        let mut offset = 0;
        loop {
            let read_size = self.read_short_dirent(|curr_ent: &ShortDirEntry| {
                curr_ent.read_at(
                    offset,
                    file_entry.as_bytes_mut(),
                    &self.fs,
                    &self.fs.read().get_fat(),
                    &self.block_device,
                )
            });
            // 读取完了
            if read_size != DIRENT_SZ || file_entry.is_empty() {
                return Some(list);
            }
            // 文件被标记删除则跳过
            if file_entry.is_deleted() {
                offset += DIRENT_SZ;
                //  is_long = false;
                continue;
            }
            if file_entry.attr() != ATTR_LONG_NAME {
                // 短文件名
                let (_, se_array, _) = unsafe { file_entry.as_bytes_mut().align_to_mut::<ShortDirEntry>() };
                let short_entry = se_array[0];
                list.push((short_entry.get_name_lowercase(), short_entry.attr()));
            } else {
                // 长文件名
                // 如果是长文件名目录项，则必是长文件名最后的那一段
                let mut name = String::new();
                let order = file_entry.order() ^ 0x40;
                for _ in 0..order {
                    name.insert_str(0, file_entry.get_name_format().as_str());
                    offset += DIRENT_SZ;
                    let read_size = self.read_short_dirent(|curr_ent: &ShortDirEntry| {
                        curr_ent.read_at(
                            offset,
                            file_entry.as_bytes_mut(),
                            &self.fs,
                            &self.fs.read().get_fat(),
                            &self.block_device,
                        )
                    });
                    if read_size != DIRENT_SZ || file_entry.is_empty() {
                    }
                }
                list.push((name.clone(), file_entry.attr()));
            }
            offset += DIRENT_SZ;
        }
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.read_short_dirent(|short_entry: &ShortDirEntry| {
            short_entry.read_at(offset, buf, &self.fs, &self.fs.read().get_fat(), &self.block_device)
        })
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        // 先扩容
        self.increase_size((offset + buf.len()) as u32);
        self.modify_short_dirent(|short_entry: &mut ShortDirEntry| {
            short_entry.write_at(offset, buf, &self.fs, &self.fs.read().get_fat(), &self.block_device)
        })
    }

    pub fn clear(&self) {
        // 难点:长名目录项也要修改
        let first_cluster: u32 = self.first_cluster();
        if self.is_dir() || first_cluster == 0 {
            return;
        }
        for i in 0..self.long_pos_vec.len() {
            self.modify_long_dirent(i, |long_entry: &mut LongDirEntry| {
                long_entry.clear();
            });
        }
        self.modify_short_dirent(|short_entry: &mut ShortDirEntry| {
            short_entry.clear();
        });
        let all_clusters = self
            .fs
            .read()
            .get_fat()
            .read()
            .get_all_cluster_of(first_cluster, self.block_device.clone());
        //self.fs.write().dealloc_cluster(all_clusters);
        let fs_reader = self.fs.read();
        fs_reader.dealloc_cluster(all_clusters);
        //fs_reader.cache_write_back();
    }

    /* 查找可用目录项，返回offset，簇不够也会返回相应的offset，caller需要及时分配 */
    fn find_free_dirent(&self) -> Option<usize> {
        if !self.is_dir() {
            return None;
        }
        let mut offset = 0;
        loop {
            let mut tmp_dirent = ShortDirEntry::new();
            let read_size = self.read_short_dirent(|short_entry: &ShortDirEntry| {
                short_entry.read_at(
                    offset,
                    tmp_dirent.as_bytes_mut(),
                    &self.fs,
                    &self.fs.read().get_fat(),
                    &self.block_device,
                )
            });
            if tmp_dirent.is_empty() || read_size == 0 {
                return Some(offset);
            }
            offset += DIRENT_SZ;
        }
    }

    // pub fn creation_time(&self) -> (u32, u32, u32, u32, u32, u32, u64) {
    //     self.read_short_dirent(|sde: &ShortDirEntry| sde.get_creation_time())
    // }

    // pub fn accessed_time(&self) -> (u32, u32, u32, u32, u32, u32, u64) {
    //     self.read_short_dirent(|sde: &ShortDirEntry| sde.get_accessed_time())
    // }

    // pub fn modification_time(&self) -> (u32, u32, u32, u32, u32, u32, u64) {
    //     self.read_short_dirent(|sde: &ShortDirEntry| sde.get_modification_time())
    // }

    /* WAITING 目前只支持删除自己*/
    pub fn remove(&self) -> usize {
        //self.modify_short_dirent(|sdent: &mut ShortDirEntry|{
        //    sdent.delete();
        //});
        let first_cluster: u32 = self.first_cluster();
        for i in 0..self.long_pos_vec.len() {
            self.modify_long_dirent(i, |long_entry: &mut LongDirEntry| {
                long_entry.delete();
            });
        }
        //println!("[fs]: rm file");
        self.modify_short_dirent(|short_entry: &mut ShortDirEntry| {
            short_entry.delete();
        });
        let all_clusters = self
            .fs
            .read()
            .get_fat()
            .read()
            .get_all_cluster_of(first_cluster, self.block_device.clone());
        self.fs.write().dealloc_cluster(all_clusters.clone());
        //self.fs.write().cache_write_back();
        return all_clusters.len();
    }

    /* WAITING */
    #[allow(unused)]
    fn remove_file() {}

    /* WAITING */
    #[allow(unused)]
    fn remove_dir() {}
}

/* WAITING */
#[allow(unused)]
pub fn fcopy() {}

/* WAITING */
#[allow(unused)]
pub fn fmove() {}
