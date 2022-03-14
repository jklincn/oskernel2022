/// # 任务控制块
/// `os/src/task/task.rs`
/// ```
/// pub struct TaskControlBlock
/// pub enum TaskStatus
/// ```
//

use super::TaskContext;
use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::trap::{trap_handler, TrapContext};

/// ### 任务控制块
pub struct TaskControlBlock {
    /// 任务状态
    pub task_status: TaskStatus,
    /// 任务上下文
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// ### 通过 elf 数据和 app_id 新建一个任务控制块
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 解析传入的 ELF 格式数据构造应用的地址空间 memory_set 并获得其他信息
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        // 从地址空间 memory_set 中查多级页表找到应用地址空间中的 Trap 上下文实际被放在哪个物理页帧
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // 为虚拟空间中的内核栈空间分配物理内存
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        // 在应用的内核栈顶压入一个跳转到 trap_return 而不是 __restore 的任务上下文
        // 这主要是为了能够支持对该应用的启动并顺利切换到用户地址空间执行
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
        };
        // 从任务控制块中获得 Trap 上下文的可变引用，然后进行初始化
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
}

/// 任务状态枚举
#[derive(Copy, Clone, PartialEq)]   // 由编译器实现一些特性
pub enum TaskStatus {
    Ready,  // 准备运行
    Running,// 正在运行
    Exited, // 已退出
}
