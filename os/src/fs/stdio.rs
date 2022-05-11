/// # 标准输入输出接口
/// `os/src/fs/stdio.rs`
/// ```
/// pub struct Stdin
/// pub struct Stdout
/// ```
//

use super::{File, Kstat};
use crate::mm::UserBuffer;
use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;
use alloc::sync::Arc;

pub use super::{list_apps, open, OSInode, OpenFlags,DiskInodeType};

pub struct Stdin;

pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        let ch = c as u8;
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }
    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }

    fn create(&self, path:&str, type_: DiskInodeType)->Option<Arc<OSInode>>{
        panic!("Stdin not implement create");
    }

    fn find(&self, path:&str, flags:OpenFlags)->Option<Arc<OSInode>>{
        panic!("Stdin not implement find");
    }

    fn get_fstat(&self, kstat:&mut Kstat){
        panic!("Stdin not implement get_fstat");
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }
    fn create(&self, path:&str, type_: DiskInodeType)->Option<Arc<OSInode>>{
        panic!("Stdout not implement create");
    }

    fn find(&self, path:&str, flags:OpenFlags)->Option<Arc<OSInode>>{
        panic!("Stdout not implement find");
    }

    fn get_fstat(&self, kstat:&mut Kstat){
        panic!("Stdout not implement get_fstat");
    }
}
