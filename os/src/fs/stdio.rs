use super::File;
use crate::mm::UserBuffer;
use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;
use alloc::vec::Vec;

pub struct Stdin;

pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn available(&self) -> bool {
        true
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

    fn get_name(&self) -> &str {
        "Stdin"
    }

    fn get_offset(&self) -> usize {
        return 0;
    }

    fn set_offset(&self, _offset: usize) {
        return;
    }

    fn file_size(&self) -> usize {
        core::usize::MAX
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn available(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        // println!("buffer:{:?}",user_buf);
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }

    fn get_name(&self) -> &str {
        "Stdout"
    }

    fn set_cloexec(&self) {
        // 涉及刚开始的 open /dev/tty，然后 sys_fcntl:fd:2,cmd:1030,arg:Some(10)
        // 可能是 sh: ls: unknown operan 等问题的原因
        // panic!("Stdput not implement set_cloexec");
    }

    fn write_kernel_space(&self, data: Vec<u8>) -> usize {
        // println!("data:{:?}",data);
        let buffer = data.as_slice();
        // println!("str:{:?}",core::str::from_utf8(buffer).unwrap());
        print!("{}", core::str::from_utf8(buffer).unwrap());
        data.len()
    }
}
