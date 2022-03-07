/// # 程序加载模块
/// `os/src/loader.rs`
/// ## 实现功能
/// ```
/// static KERNEL_STACK: [KernelStack; MAX_APP_NUM]
/// static USER_STACK: [UserStack; MAX_APP_NUM]
/// pub fn get_num_app() -> usize
/// pub fn load_apps()
/// pub fn init_app_cx(app_id: usize) -> usize
/// ```
//

use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    /// 获取栈顶地址
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    /// 将上下文推入栈
    pub fn push_context(&self, trap_cx: TrapContext) -> usize {
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe { *trap_cx_ptr = trap_cx; }
        trap_cx_ptr as usize
    }
}

impl UserStack {
    /// 获取栈顶地址
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/// 获取第 `app_id` 个应用的起始地址 `BASE_ADDRESS`
fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// 读出 `link_app.S` 中 `_num_app` 中第一个元素即app数量
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    // 把_num_app从地址转换为指针，然后根据指针读出数据
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// 从内核数据区 `.data` 加载应用程序到起始地址 `BASE_ADDRESS`(由 `user/src/linker.ld` 定义)
pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    // 读出从 _num_app 数组中的地址信息(从第二个元素开始的num_app + 1个元素)
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    unsafe { asm!("fence.i"); } // 清理 指令缓存(i-cache)
    // 循环加载每个应用
    for i in 0..num_app {
        let base_i = get_base_i(i);
        // 应用程序内存空间清零
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
        // 从数据区读出应用程序数据
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        // 获取待写入区域的可变切片
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        dst.copy_from_slice(src);
    }
}

/// 初始化第 `app_id` 个应用程序的 Trap 上下文
pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_i(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}
