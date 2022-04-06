/// # 与虚拟机相关的参数
/// `os/src/boards/qemu.rs`
//

pub const CLOCK_FREQ: usize = 12500000;

pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
