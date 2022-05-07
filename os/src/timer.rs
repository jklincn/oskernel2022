/// # 时间模块
/// `os/src/timer.rs`
/// ## 实现功能
/// ```
/// pub struct  TimeVal
/// pub fn get_time() -> usize
/// pub fn get_time_ms() -> usize
/// pub fn get_TimeVal() -> TimeVal
/// pub fn set_next_trigger()
/// ```
//

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

/// ### Linux 时间格式
/// - `sec`：秒
/// - `usec`：微秒
/// - 两个值相加的结果是结构体表示的时间
pub struct  TimeVal {
    /// 单位：秒
    pub sec:usize,  /// 单位：微秒
    pub usec:usize,
}

/// ### 取得当前 `mtime` 计数器的值
/// - `mtime`: 统计处理器自上电以来经过了多少个内置时钟的时钟周期,64bit
pub fn get_time() -> usize {
    time::read()
}

/// 获取CPU上电时间（单位：ms）
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

/// 获取 `TimeVal` 格式的时间信息
#[allow(non_snake_case)]
pub fn get_TimeVal() -> TimeVal{
    let time_ms = get_time_ms();
    TimeVal {
        sec: time_ms / 1000,
        usec: (time_ms % 1000) * 1000,
    }
}

/// ### 设置下次触发时钟中断的时间
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
