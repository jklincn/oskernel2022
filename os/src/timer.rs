/// # 时间模块
/// `os/src/timer.rs`
/// ## 实现功能
/// ```
/// pub struct  TimeVal
/// pub fn get_time() -> usize
/// pub fn get_time_ms() -> usize
/// pub fn get_timeval() -> TimeVal
/// pub fn set_next_trigger()
/// ```
//

use core::ops::{Add, Sub};

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

pub const TICKS_PER_SEC: usize = 100;
pub const MSEC_PER_SEC: usize = 1000;
pub const USEC_PER_SEC: usize = 1000_000;
pub const NSEC_PER_SEC: usize = 1000_000_000;

/// ### Linux 时间格式
/// - `sec`：秒
/// - `usec`：微秒
/// - 两个值相加的结果是结构体表示的时间
#[derive(Copy, Clone)]
pub struct  TimeVal {
    /// 单位：秒
    pub sec:usize,  /// 单位：微秒
    pub usec:usize,
}

impl Add for TimeVal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut sec = self.sec + other.sec;
        let mut usec = self.usec + other.usec;
        sec += usec/USEC_PER_SEC;
        usec %= USEC_PER_SEC;
        Self {
            sec,
            usec,
        }
    }
}

impl Sub for TimeVal {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.sec < other.sec{
            return Self{sec:0,usec:0}
        }
        else if self.sec == other.sec{
            if self.usec < other.usec{
                return Self{sec:0,usec:0}
            }
            else{
                return Self{sec:0,usec:self.usec-other.usec}
            }
        }
        else{
            let mut sec = self.sec - other.sec;
            let mut usec = self.usec - other.usec;
            if self.usec < other.usec{
                sec -= 1;
                usec = USEC_PER_SEC + self.usec - other.usec;
            }
            Self {
                sec,
                usec,
            }
        }
    }
}

impl TimeVal {
    pub fn new() -> Self{
        Self{
            sec:0,
            usec:0
        }
    }

    pub fn is_zero(&self) -> bool{
        self.sec == 0 && self.usec == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

#[allow(non_camel_case_types)]
/// ### Linux 间隔计数
/// - `tms_utime`：用户态时间
/// - `tms_stime`：内核态时间
/// - `tms_cutime`：已回收子进程的用户态时间
/// - `tms_cstime`：已回收子进程的内核态时间
pub struct tms {    /// 用户态时间
    pub tms_utime:isize,    /// 内核态时间
    pub tms_stime:isize,    /// 已回收子进程的用户态时间
    pub tms_cutime:isize,   /// 已回收子进程的内核态时间
    pub tms_cstime:isize,
}

impl tms {
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
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

#[allow(unused)]
pub fn get_time_ns() -> usize {
    (get_time() / (CLOCK_FREQ / USEC_PER_SEC)) * MSEC_PER_SEC
}

#[allow(unused)]
pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / USEC_PER_SEC) 
}

#[allow(unused)]
pub fn get_time_s() -> usize {
    get_time() / CLOCK_FREQ
}

/// 获取 `TimeVal` 格式的时间信息
pub fn get_timeval() -> TimeVal {
    let ticks = get_time();
    let sec = ticks/CLOCK_FREQ;
    let usec = (ticks%CLOCK_FREQ) * USEC_PER_SEC / CLOCK_FREQ;
    TimeVal{
        sec,
        usec
    }
}

/// ### 设置下次触发时钟中断的时间
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub struct Timespec {
    pub tv_sec: u64,  // 秒
    pub tv_nsec: u64, // 纳秒
}