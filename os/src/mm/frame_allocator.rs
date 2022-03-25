// 主要是提供 frame_alloc() 与 frame_dealloc() 两个接口

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

// 定义一个物理页帧管理器需要提供的功能（即特性）
trait FrameAllocator {
    // 创建实例
    fn new() -> Self;
    // 分配物理页帧
    fn alloc(&mut self) -> Option<PhysPageNum>;
    // 回收物理页帧
    fn dealloc(&mut self, ppn: PhysPageNum);
}

// 栈式物理页帧管理器
pub struct StackFrameAllocator {
    current: usize,       // 空闲内存的起始物理页号
    end: usize,           // 空闲内存的结束物理页号
    recycled: Vec<usize>, // 物理页帧回收栈
}

// 为栈式物理页帧管理器实现三个特性
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        // 首先检查回收栈中是否为空，如果不为空则弹出栈顶并返回（转换为 PhysPageNum ）
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        }
        // 检查内存空间是否已满，如果已满则返回 None
        else if self.current == self.end {
            None
        }
        // 把最小的空闲物理页号分配出去
        else {
            self.current += 1; //表示 current 已被分配
            Some((self.current - 1).into())
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        //此时 ppn 是 usize 类型
        let ppn = ppn.0;
        // 验证物理页号的合法性：若PPN大于已分配的最高内存或回收栈中存在这个物理页号，则出错
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // 回收（压栈）
        self.recycled.push(ppn);
    }
}

// 栈式物理页帧管理器方法
impl StackFrameAllocator {
    // 初始化栈式物理页管理器
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        println!("last {} Physical Frames.", self.end - self.current);
    }
}

// 取别名
type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    // 物理页帧管理器全局实例
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
    unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

// 初始化物理页帧管理器
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    // 调用 PhysAddr 的 floor/ceil 方法分别下/上取整获得可用的物理页号区间
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

// 物理页帧
// 借用了 RAII 的思想，将一个物理页帧的生命周期绑定到一个 FrameTracker 变量上
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

// 物理页帧方法
impl FrameTracker {
    // 通过物理页号创建一个物理页帧的结构体，创建时初始化内存空间
    pub fn new(ppn: PhysPageNum) -> Self {
        // 物理页清零
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

// 公开给其他内核模块调用的分配物理页帧的接口
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new) // 在这里 map() 将 Option<PhysPageNum> 类型转换为 Option<FrameTracker> 类型
}

// 公开给其他内核模块调用的回收物理页帧的接口
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

// 为物理页帧实现 Debug 特性
impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

// 为物理页帧实现 Drop 特性
impl Drop for FrameTracker {
    /// 当一个 FrameTracker 生命周期结束被编译器回收的时候，我们需要将它控制的物理页帧回收掉
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}
