use core::any::Any;

/// 一个块设备的抽象接口
/// 由具体的块设备驱动来实现这两个方法（内核或其他应用程序）
pub trait BlockDevice: Send + Sync + Any {
    /// 将编号为 block_id 的块从磁盘读入内存中的缓冲区 buf
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    /// 将内存中的缓冲区 buf 中的数据写入磁盘编号为 block_id 的块
    fn write_block(&self, block_id: usize, buf: &[u8]);
    fn handle_irq(&self);
}
