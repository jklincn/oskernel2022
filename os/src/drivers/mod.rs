/// # 设备驱动层
/// `os/src/drivers/mod.rs`
/// ```
/// pub use block::BLOCK_DEVICE
/// ```
//

pub mod block;

pub use block::BLOCK_DEVICE;
