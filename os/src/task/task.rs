/// # 进程控制块
/// `os/src/task/task.rs`
/// ```
/// pub struct TaskControlBlock
/// pub struct TaskControlBlockInner
/// pub enum TaskStatus
/// ```
//

use super::TaskContext;
use super::{pid_alloc, KernelStack, PidHandle, SignalFlags};
use crate::config::*;
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{translated_refmut, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE, MmapArea, MapPermission};
use crate::sync::UPSafeCell;
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;

/// ### 进程控制块
/// |成员变量|描述|
/// |--|--|
/// |`pid`|进程标识符|
/// |`kernel_stack`|应用内核栈|
/// |`inner`|全局可变部分|
/// ```
/// TaskControlBlock::inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner>
/// TaskControlBlock::getpid(&self) -> usize
/// TaskControlBlock::new(elf_data: &[u8]) -> Self
/// TaskControlBlock::exec(&self, elf_data: &[u8])
/// TaskControlBlock::fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock>
/// ```
pub struct TaskControlBlock {
    /// 进程标识符
    pub pid: PidHandle,
    /// thread group id
    pub tgid: usize,
    /// 应用内核栈
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<TaskControlBlockInner>,
}

/// ### 进程控制块中可能发生变化的数据
/// |成员变量|描述|
/// |--|--|
/// |`trap_cx_ppn`|应用地址空间中的 Trap 上下文所在的物理页帧的物理页号|
/// |`base_size`|应用数据仅有可能出现在应用地址空间低于 base_size 字节的区域中。<br>借助它我们可以清楚的知道应用有多少数据驻留在内存中|
/// |`task_cx`|任务上下文|
/// |`task_status`|维护当前进程的执行状态|
/// |`memory_set`|应用地址空间|
/// |`parent`|指向当前进程的父进程（如果存在的话）|
/// |`children`|当前进程的所有子进程的任务控制块向量|
/// |`exit_code`|退出码|
/// |`fd_table`|文件描述符表|
/// 
/// 注意我们在维护父子进程关系的时候大量用到了引用计数 `Arc/Weak` 。进程控制块的本体是被放到内核堆上面的，
/// 对于它的一切访问都是通过智能指针 `Arc/Weak` 来进行的，这样是便于建立父子进程的双向链接关系（避免仅基于 `Arc` 形成环状链接关系）。
/// 当且仅当智能指针 `Arc` 的引用计数变为 0 的时候，进程控制块以及被绑定到它上面的各类资源才会被回收。
/// 子进程的进程控制块并不会被直接放到父进程控制块中，因为子进程完全有可能在父进程退出后仍然存在。
/// ```
/// TaskControlBlockInner::get_trap_cx(&self) -> &'static mut TrapContext
/// TaskControlBlockInner::get_user_token(&self) -> usize
/// TaskControlBlockInner::is_zombie(&self) -> bool
/// ```
pub struct TaskControlBlockInner {
    // 进程
    /// 应用地址空间中的 Trap 上下文所在的物理页帧的物理页号
    pub trap_cx_ppn: PhysPageNum,
    /// 任务上下文
    pub task_cx: TaskContext,
    /// 维护当前进程的执行状态
    pub task_status: TaskStatus,
    /// 指向当前进程的父进程（如果存在的话）
    pub parent: Option<Weak<TaskControlBlock>>, 
    /// 当前进程的所有子进程的任务控制块向量
    pub children: Vec<Arc<TaskControlBlock>>,   
    /// 退出码
    pub exit_code: i32,  

    // 内存
    /// 应用数据仅有可能出现在应用地址空间低于 base_size 字节的区域中。
    /// 借助它我们可以清楚的知道应用有多少数据驻留在内存中
    pub base_size: usize,           
    /// 应用地址空间
    pub memory_set: MemorySet,
    // 虚拟内存地址映射空间
    pub mmap_area: MmapArea,
    pub heap_start: usize,
    pub heap_pt: usize,
    
    // 文件
    /// 文件描述符表
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,

    // 状态信息
    pub signals: SignalFlags,
    pub current_path: String,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// 获取用户地址空间的 token (符合 satp CSR 格式要求的多级页表的根节点所在的物理页号)
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
    /// ### 查找空闲文件描述符下标
    /// 从文件描述符表中 **由低到高** 查找空位，返回向量下标，没有空位则在最后插入一个空位
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    pub fn get_work_path(&self)->String{
        self.current_path.clone()
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    /// 通过 elf 数据新建一个任务控制块，目前仅用于内核中手动创建唯一一个初始进程 initproc
    pub fn new(elf_data: &[u8]) -> Self {
        // 解析传入的 ELF 格式数据构造应用的地址空间 memory_set 并获得其他信息
        let (memory_set, user_sp, user_heap, entry_point) = MemorySet::from_elf(elf_data);
        // 从地址空间 memory_set 中查多级页表找到应用地址空间中的 Trap 上下文实际被放在哪个物理页帧
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 为进程分配 PID 以及内核栈，并记录下内核栈在内核地址空间的位置
        let pid_handle = pid_alloc();
        let tgid = pid_handle.0;
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // 在该进程的内核栈上压入初始化的任务上下文，使得第一次任务切换到它的时候可以跳转到 trap_return 并进入用户态开始执行
        let task_control_block = Self {
            pid: pid_handle,
            tgid,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    heap_start: user_heap,
                    heap_pt: user_heap,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    current_path:String::from("/"),
                    mmap_area: MmapArea::new(VirtAddr::from(MMAP_BASE), VirtAddr::from(MMAP_BASE)),
                })
            },
        };
        // 初始化位于该进程应用地址空间中的 Trap 上下文，使得第一次进入用户态的时候时候能正
        // 确跳转到应用入口点并设置好用户栈，同时也保证在 Trap 的时候用户态能正确进入内核态
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    /// 用来实现 exec 系统调用，即当前进程加载并执行另一个 ELF 格式可执行文件
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        // 从 ELF 文件生成一个全新的地址空间并直接替换
        let (memory_set, mut user_sp,user_heap, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // 将命令行参数以某种格式压入用户栈
        // 在用户栈中提前分配好 参数字符串首地址 的空间
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;                             // 参数字符串首地址数组 起始地址
        let mut argv: Vec<_> = (0..=args.len()) // 获取 参数字符串首地址数组 在用户栈中的可变引用
            .map(|arg| {
                translated_refmut(
                    memory_set.token(),
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
            
        // 参数字符串
        *argv[args.len()] = 0;  // 标记参数尾
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;   // 在用户栈中分配 参数i 的空间
            *argv[i] = user_sp;             // 在 参数字符串首地址数组中 记录 参数i 首地址
            let mut p = user_sp;
            for c in args[i].as_bytes() {   // 将参数写入到用户栈
                *translated_refmut(memory_set.token(), p as *mut u8) = *c;
                p += 1;
            }                                    // 写入字符串结束标记
            *translated_refmut(memory_set.token(), p as *mut u8) = 0;
        }

        // 将 user_sp 以 8 字节对齐，这是因为命令行参数的长度不一，很有可能压入之后 user_sp 没有对齐到 8 字节，
        // 那么在 K210 平台上在访问用户栈的时候就会触发访存不对齐的异常.在 Qemu 平台上则并不存在这个问题。
        user_sp -= user_sp % core::mem::size_of::<usize>();

        let mut inner = self.inner_exclusive_access();
        inner.memory_set = memory_set;  // 这将导致原有的地址空间生命周期结束，里面包含的全部物理页帧都会被回收
        inner.heap_start = user_heap;
        inner.heap_pt = user_heap;
        inner.trap_cx_ppn = trap_cx_ppn;
        let trap_cx = inner.get_trap_cx();
        // 修改新的地址空间中的 Trap 上下文，将解析得到的应用入口点、用户栈位置以及一些内核的信息进行初始化
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        // 修改 Trap 上下文中的 a0/a1 寄存器
        trap_cx.x[10] = args.len(); // a0 表示命令行参数的个数
        trap_cx.x[11] = argv_base;  // a1 则表示 参数字符串首地址数组 的起始地址
    }
    
    /// 用来实现 fork 系统调用，即当前进程 fork 出来一个与之几乎相同的子进程
    pub fn fork(self: &Arc<TaskControlBlock>, is_create_thread: bool) -> Arc<TaskControlBlock> {
        let mut parent_inner = self.inner_exclusive_access();
        // 拷贝用户地址空间
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 分配一个 PID
        let pid_handle = pid_alloc();
        let mut tgid = 0;
        if is_create_thread{
            tgid = self.pid.0;
        }
        else{
            tgid = pid_handle.0;
        }
        // 根据 PID 创建一个应用内核栈
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();

        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }

        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            tgid,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    heap_start: parent_inner.heap_start,
                    heap_pt: parent_inner.heap_pt,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    current_path: parent_inner.current_path.clone(),
                    mmap_area: MmapArea::new(VirtAddr::from(MMAP_BASE), VirtAddr::from(MMAP_BASE)),
                })
            },
        });
        // 把新生成的进程加入到子进程向量中
        parent_inner.children.push(task_control_block.clone());
        // 更新子进程 trap 上下文中的栈顶指针
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
    }

    /// ### 尝试用时加载缺页，目前只支持mmap缺页
    /// - 参数：
    ///     - `va`：缺页中的虚拟地址
    ///     - `is_load`：加载(1)/写入(0)
    /// - 返回值：
    ///     - `0`：成功加载缺页
    ///     - `-1`：加载缺页失败
    pub fn check_lazy(&self, va: VirtAddr, is_load: bool) -> isize {
        let inner = self.inner_exclusive_access();
        //let heap_base = inner.heap_start;
        //let heap_pt = inner.heap_pt;
        //let stack_top = inner.base_size;
        //let stack_bottom = stack_top - USER_STACK_SIZE;
        let mmap_start = inner.mmap_area.mmap_start;
        let mmap_end = inner.mmap_area.mmap_top;
        drop(inner);
        
        if va >= mmap_start && va < mmap_end {
            //println!("lazy mmap");
            self.lazy_mmap(va.0, is_load)
        } 
        else { 
            println!("va: 0x{:x}", va.0);
            println!("mmap_start: 0x{:x}", mmap_start.0);
            println!("mmap_end: 0x{:x}", mmap_end.0);
            -2 
        }
        
        // else if va.0 >= heap_base && va.0 <= heap_pt {
        //     inner.lazy_alloc_heap(vpn);
        //     return 0;
        // } else if va.0 >= stack_bottom && va.0 <= stack_top {
        //     //println!{"lazy_stack_page: {:?}", va}
        //     inner.lazy_alloc_stack(vpn);
        //     0
        // } else {
        //     // get the PageTableEntry that faults
        //     let pte = inner.enquire_vpn(vpn);
        //     // if the virtPage is a CoW
        //     if pte.is_some() && pte.unwrap().is_cow() {
        //         let former_ppn = pte.unwrap().ppn();
        //         inner.cow_alloc(vpn, former_ppn);
        //         0
        //     } else {
        //         -1
        //     }
        // }
    }

    /// ### 用时加载mmap缺页
    /// - 参数：
    ///     - `stval`：缺页中的虚拟地址
    ///     - `is_load`：加载(1)/写入(0)
    /// - 返回值：
    ///     - `0`
    ///     - `-1`
    pub fn lazy_mmap(&self, stval: usize, is_load: bool) -> isize {
        let mut inner = self.inner_exclusive_access();
        let fd_table = inner.fd_table.clone();
        let token = inner.get_user_token();
        let lazy_result = inner.memory_set.lazy_mmap(stval.into());

        if lazy_result == 0 || is_load {
            inner.mmap_area.lazy_map_page(stval, fd_table, token);
        }
        return lazy_result;
    }
    
    /// ### 在进程虚拟地址空间中分配创建一片虚拟内存地址映射
    /// - 参数
    ///     - `start`, `len`：映射空间起始地址及长度，起始地址必须4k对齐
    ///     - `prot`：映射空间读写权限
    ///         ```c
    ///         #define PROT_NONE  0b0000
    ///         #define PROT_READ  0b0001
    ///         #define PROT_WRITE 0b0010
    ///         #define PROT_EXEC  0b0100
    ///         ```
    ///     - `flags`：映射方式
    ///         ```rust
    ///         const MAP_FILE = 0;
    ///         const MAP_SHARED= 0x01;
    ///         const MAP_PRIVATE = 0x02;
    ///         const MAP_FIXED = 0x10;
    ///         const MAP_ANONYMOUS = 0x20;
    ///         ```
    ///     - `fd`：映射文件描述符
    ///     - `off`: 偏移量
    /// - 返回值：从文件的哪个位置开始映射
    pub fn mmap(&self, start: usize, len: usize, prot: usize, flags: usize, fd: isize, off: usize) -> usize {        
        if start % PAGE_SIZE != 0{
            panic!("mmap: start_va not aligned");
        }

        let mut inner = self.inner_exclusive_access();
        let fd_table = inner.fd_table.clone();
        let token = inner.get_user_token();
        let va_top = inner.mmap_area.get_mmap_top();
        let end_va = VirtAddr::from(va_top.0 + len);

        // "prot<<1" is equal to meaning of "MapPermission"
        // "1<<4" means user
        let map_flags = (((prot & 0b111)<<1) + (1<<4))  as u8;
    
        let mut startvpn = start/PAGE_SIZE;
        
        if start != 0 { // "Start" va Already mapped
            while startvpn < (start + len) / PAGE_SIZE {
                if inner.memory_set.set_pte_flags(startvpn.into(), map_flags as usize) == -1{
                    panic!("mmap: start_va not mmaped");
                }
                startvpn +=1;
            }
            return start;
        }
        else{ // "Start" va not mapped
            inner.memory_set.insert_mmap_area(va_top, end_va, MapPermission::from_bits(map_flags).unwrap());

            // 创建mmap后直接加载一页，不使用lazy mmap
            inner.memory_set.lazy_mmap(va_top);
            inner.mmap_area.push(va_top.0, len, prot, flags, fd, off, fd_table, token);
            // println!("ppn: 0x{:x}", inner.memory_set.translate(va_top.into()).unwrap().ppn().0);
            // inner.memory_set.debug_show_data(va_top);
            //-------------------------------------

            drop(inner);
            // super::processor::current_task().unwrap().check_lazy(va_top, true);
            // self.check_lazy(va_top, true);

            va_top.0
        }
    }

    pub fn munmap(&self, start: usize, len: usize) -> isize {
        let mut inner = self.inner_exclusive_access();
        inner.memory_set.remove_area_with_start_vpn(VirtAddr::from(start).into());
        inner.mmap_area.remove(start, len)
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    pub fn get_parent(&self) -> Option<Arc<TaskControlBlock>> {
        let inner = self.inner.exclusive_access();
        inner.parent.as_ref().unwrap().upgrade()
    }

    pub fn grow_proc(&self, grow_size: isize) -> usize {
        if grow_size > 0 {
            let growed_addr: usize = self.inner.exclusive_access().heap_pt + grow_size as usize;
            let limit = self.inner.exclusive_access().heap_start + USER_HEAP_SIZE;
            if growed_addr > limit {
                panic!("process doesn't have enough memsize to grow! {} {} {} {}", limit, self.inner.exclusive_access().heap_pt, growed_addr, self.pid.0);
            }
            self.inner.exclusive_access().heap_pt = growed_addr;
        }
        else {
            let shrinked_addr: usize = self.inner.exclusive_access().heap_pt + grow_size as usize;
            if shrinked_addr < self.inner.exclusive_access().heap_start {
                panic!("Memory shrinked to the lowest boundary!")
            }
            self.inner.exclusive_access().heap_pt = shrinked_addr;
        }
        return self.inner.exclusive_access().heap_pt;
    }
}

/// ### 任务状态枚举
/// |状态|描述|
/// |--|--|
/// |`Ready`|准备运行|
/// |`Running`|正在运行|
/// |`Zombie`|僵尸态|
#[derive(Copy, Clone, PartialEq)]   // 由编译器实现一些特性
pub enum TaskStatus {
    Ready,  // 准备运行
    Running,// 正在运行
    Zombie, // 僵尸态
}
