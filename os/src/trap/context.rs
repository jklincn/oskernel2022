/// # `Trap` 上下文模块
/// `os/src/trap/context.rs`
/// 
/// 提供 `Trap`上下文结构体
/// ```
/// pub struct TrapContext
/// TrapContext::set_sp()
/// TrapContext::app_init_context(entry: usize, sp: usize) -> Self()
/// ```
//

use riscv::register::sstatus::{self, Sstatus, SPP};

/// ### `Trap` 上下文的拷贝
/// 保存以下数据
/// - 通用寄存器`x[0]~x[31]`
/// - `sstatus`
/// - `sepc` 程序跳转地址
/// 
/// 提供以下函数
/// ```
/// pub fn set_sp(&mut self, sp: usize)
/// pub fn app_init_context(entry: usize, sp: usize) -> Self
/// ```
//
#[repr(C)]
pub struct TrapContext {
    /// 通用寄存器`x[0]~x[31]`
    pub x: [usize; 32],
    /// 提供状态信息
    pub sstatus: Sstatus,
    /// 记录 Trap 发生之前执行的最后一条指令的地址
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// 初始化上下文 
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }
}
