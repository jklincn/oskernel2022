use alloc::vec::Vec;
use crate::config::PAGE_SIZE;

#[derive(Clone, Copy,Debug)]
pub struct AuxEntry(pub usize, pub usize);

pub const AT_NULL: usize = 0;
pub const AT_PHDR: usize = 3;
pub const AT_PHENT: usize = 4;
pub const AT_PHNUM: usize = 5;
pub const AT_PAGESZ: usize = 6;
pub const AT_BASE: usize = 7;
pub const AT_ENTRY: usize = 9;
pub const AT_UID: usize = 11;
pub const AT_EUID: usize = 12;
pub const AT_GID: usize = 13;
pub const AT_EGID: usize = 14;
pub const AT_SECURE: usize = 23;
pub const AT_RANDOM: usize = 25;

pub fn new() -> Vec<AuxEntry> {
    let mut temp = Vec::new();
    temp.push(AuxEntry(AT_NULL, 0));
    temp.push(AuxEntry(AT_PAGESZ, PAGE_SIZE));
    temp.push(AuxEntry(AT_UID, 0));
    temp.push(AuxEntry(AT_EUID, 0));
    temp.push(AuxEntry(AT_GID, 0));
    temp.push(AuxEntry(AT_EGID, 0));
    temp.push(AuxEntry(AT_SECURE, 0));
    temp
}
