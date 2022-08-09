/// # VirtIO 总线架构下的块设备
/// `os/src/drivers/block/virtio_blk.rs`
/// ```
/// pub struct VirtIOBlock
/// pub fn virtio_dma_alloc ()
/// pub fn virtio_dma_dealloc ()
/// pub fn virtio_phys_to_virt ()
/// pub fn virtio_virt_to_phys ()
/// ```
//

use super::BlockDevice;
use crate::mm::{
    frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    StepByOne, VirtAddr,
};
use spin::Mutex;
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = unsafe { Mutex::new(Vec::new()) };
}

/// ### VirtIO 总线架构下的块设备
/// 将 `virtio-drivers` crate 提供的 VirtIO 块设备抽象 `VirtIOBlk` 包装为我们自己的 `VirtIOBlock` ，
/// 实质上只是加上了一层互斥锁，生成一个新的类型来实现 easy-fs 需要的 `BlockDevice` Trait
/// ```
/// fn read_block()
/// fn write_block()
/// pub fn new()
/// ```
pub struct VirtIOBlock(Mutex<VirtIOBlk<'static>>);

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        // println!("block_id:{}",block_id);
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
    fn handle_irq(&self) { todo!() }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(Mutex::new(
                // VirtIOHeader 实际上就代表以 MMIO 方式访问 VirtIO 设备所需的一组设备寄存器
                VirtIOBlk::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.lock().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PageTable::from_token(kernel_token())
        .translate_va(vaddr)
        .unwrap()
}
