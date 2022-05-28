use alloc::{string::String, sync::Arc, vec::Vec};
use lazy_static::*;
use spin::Mutex;

const MNT_MAXLEN: usize = 16;

pub struct MountTable {
    mnt_list: Vec<(String, String, String)>, // special, dir, fstype
}

impl MountTable {
    pub fn mount(&mut self, special: String, dir: String, fstype: String, flags: u32) -> isize {
        if self.mnt_list.len() == MNT_MAXLEN {
            return -1;
        }
        // 已存在
        if self.mnt_list.iter().find(|&(_, d, _)| *d == dir).is_some() {
            return 0;
        }

        // todo
        _ = flags;

        self.mnt_list.push((special, dir, fstype));
        0
    }

    pub fn umount(&mut self, special: String, flags: u32) -> isize {
        let len = self.mnt_list.len();

        // todo
        _ = flags;

        for i in 0..len {
            // 根据系统调用规范应该是 self.mnt_list[i].0 == special
            // 然而测试程序传的是 dir，因此这里加了一个或运算
            if self.mnt_list[i].0 == special || self.mnt_list[i].1 == special {
                self.mnt_list.remove(i);
                return 0;
            }
        }
        -1
    }
}

lazy_static! {
    pub static ref MNT_TABLE: Arc<Mutex<MountTable>> = {
        let mnt_table = MountTable { mnt_list: Vec::new() };
        Arc::new(Mutex::new(mnt_table))
    };
}
