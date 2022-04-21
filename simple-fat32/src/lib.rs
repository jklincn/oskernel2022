#![no_std]
extern crate alloc;

mod block_cache;
mod block_dev;
mod fat32_manager;
mod layout;
mod vfs;

pub const BLOCK_SZ: usize = 512;
pub use block_dev::BlockDevice;
pub use layout::ShortDirEntry;
pub use vfs::VFile;
//pub use layout::NAME_LENGTH_LIMIT;
use block_cache::{get_block_cache, get_info_cache, set_start_sec, write_to_dev, CacheMode};
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
