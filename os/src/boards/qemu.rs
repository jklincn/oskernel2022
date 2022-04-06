/// # 与虚拟机相关的参数
/// `os/src/boards/qemu.rs`
//

pub const CLOCK_FREQ: usize = 12500000;

/// 硬编码 Qemu 上的 VirtIO 总线的 MMIO 地址区间（起始地址，长度）
pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
