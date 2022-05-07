/// # 控制台模块
/// `os/src/main.rs`
/// ## 功能
/// - 提供基于Stdout结构体的标准输出宏
/// ```
/// pub fn print(args: fmt::Arguments)
/// macro_rules! print
/// macro_rules! println
/// ```
//

use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;      //类单元结构体，用于格式化输出

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

/// 采用Stdout结构体的方式向终端输出
pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

/// 输出到终端的宏打印，采用Stdout结构体
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

/// 输出到终端的宏打印(待换行符)，采用Stdout结构体
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
