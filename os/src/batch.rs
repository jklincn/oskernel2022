/// # 简单的批处理系统
/// `os/src/batch.rs`
/// ## 实现功能
/// - 用户栈 `UserStack` 和 内核栈 `KernelStack`
/// - 应用程序管理器 `AppManager`
/// ```
/// AppManager::init()
/// AppManager::print_app_info()
/// AppManager::run_next_app()
/// ```
//

use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::*;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl KernelStack {
    /// 获取栈顶地址
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    /// 将上下文推入栈
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        // 寻找放置上下文的最低地址，从栈顶指针减去所需空间（RISC-V栈空间分配规则）
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe { *cx_ptr = cx; }    // 写入上下文到栈中
        unsafe { cx_ptr.as_mut().unwrap() } // 暂未了解
    }
}

impl UserStack {
    /// 获取栈顶地址
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/// ### 应用管理器
/// - 成员
///     - current_app 字段表示当前执行的是第几个应用
///     - num_app 保存应用数量
///     - app_start 保存应用程序开始位置信息
struct AppManager {
    /// 保存应用数量
    num_app: usize,
    /// 当前执行的是第几个应用
    current_app: usize,
    /// 保存应用程序开始位置信息
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    /// 打印当前程序信息
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    /// 这个方法负责将参数 `app_id` 对应的应用程序的二进制镜像加载到物理内存以 `0x80400000` 起始的位置
    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            panic!("All applications completed!");
        }
        println!("[kernel] Loading app_{}", app_id);
        asm!("fence.i");    // 清理 指令缓存(i-cache)
        // 将一块内存清空
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
    }

    /// 获取当前运行程序的序号
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    /// 返回下一个程序的序号，且当前序号加一
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

// ### 初始化AppManager
// 找到 `link_app.S` 中提供的符号 `_num_app` ，并从这里开始解析出应用数量以及各个应用的起始地址
// 这里我们使用了外部库 lazy_static 提供的 lazy_static! 宏。要引入这个外部库，我们需要在 os/Cargo.toml 加入依赖
lazy_static! {  
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();
            }
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw: &[usize] =
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManager {
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

/// 初始化App_Manager
pub fn init() {
    //在调用 print_app_info 的时候第一次用到了全局变量 APP_MANAGER ，它也是在这个时候完成初始化
    print_app_info();
}

/// 打印当前程序信息
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// 加载并运行下一个应用程序
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);  // 释放app_manager
    // 在此之前，我们必须手动删除与资源相关的局部变量并释放资源
    extern "C" {
        fn __restore(cx_addr: usize);
    }
    // 恢复上下文
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}
