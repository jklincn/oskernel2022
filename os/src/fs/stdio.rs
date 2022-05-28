/// # 标准输入输出接口
/// `os/src/fs/stdio.rs`
/// ```
/// pub struct Stdin
/// pub struct Stdout
/// ```
//

use super::{File, Kstat, Dirent};
use crate::mm::UserBuffer;
use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;
use alloc::string::String;

pub use super::{list_apps, open, OSInode, OpenFlags};

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

    #[allow(unused_variables)]
    fn get_fstat(&self, kstat: &mut Kstat) {
        panic!("Stdin not implement get_fstat");
    }

    #[allow(unused_variables)]
    fn get_dirent(&self, dirent: &mut Dirent) -> isize {
        panic!("Stdin not implement get_dirent");
    }

    fn get_name(&self) -> String {
        panic!("Stdin not implement get_name");
    }

    fn set_offset(&self, offset: usize) {
        panic!("Stdin not implement set_offset");
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

    #[allow(unused_variables)]
    fn get_fstat(&self, kstat: &mut Kstat) {
        panic!("Stdout not implement get_fstat");
    }

    #[allow(unused_variables)]
    fn get_dirent(&self, dirent: &mut Dirent) -> isize {
        panic!("Stdout not implement get_dirent");
    }

    fn get_name(&self) -> String {
        panic!("Stdout not implement get_name");
    }

    fn set_offset(&self, offset: usize) {
        panic!("Stdput not implement set_offset");
    }
}
