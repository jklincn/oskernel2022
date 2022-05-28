#![no_std]
extern crate alloc;

#[macro_use]
mod console;
mod sbi;
mod block_cache;
mod block_dev;
mod fat32_manager;
mod layout;
mod vfs;


pub const BLOCK_SIZE: usize = 512;
pub use block_dev::BlockDevice;
pub use layout::ShortDirEntry;
pub use vfs::{VFile,create_root_vfile};
use block_cache::{get_block_cache, set_start_sec, write_to_dev};
pub use fat32_manager::FAT32Manager;
pub use layout::*;

pub fn clone_into_array<A, T>(slice: &[T]) -> A
where
    A: Default + AsMut<[T]>,
    T: Clone,
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}
