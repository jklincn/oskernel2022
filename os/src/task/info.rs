use crate::sync::UPSafeCell;
/// # 系统信息模块
/// `os/src/task/info.rs`
/// ```
/// pub struct Utsname
/// ```
//
//use alloc::sync::Arc;

use lazy_static::*;

pub struct Utsname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

lazy_static! {
    pub static ref UTSNAME: UPSafeCell<Utsname> = unsafe {UPSafeCell::new(Utsname::new())};
}

impl Utsname {
    pub fn new() -> Self {
        Self {
            sysname: Utsname::str2u8("Linux"),
            nodename: Utsname::str2u8("untuntu"),
            release: Utsname::str2u8("20220421"),
            version: Utsname::str2u8("5.13"),
            machine: Utsname::str2u8("riscv64"),
            domainname: Utsname::str2u8("Jeremy_test"),
        }
    }

    pub fn str2u8(str: &str) -> [u8; 65] {
        let mut arr: [u8; 65] = [0; 65];
        let cstr = str.as_bytes();
        let len = str.len();
        for i in 0..len{
            arr[i] = cstr[i];
        }
        arr
    }
}
