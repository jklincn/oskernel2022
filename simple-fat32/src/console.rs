use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

macro_rules! print {
        ($fmt: literal $(, $($arg: tt)+)?) => {
            $crate::console::print(format_args!($fmt $(, $($arg)+)?))
        }
    }

macro_rules! println {
        ($fmt: literal $(, $($arg: tt)+)?) => {
            $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
        }
    }

macro_rules! color_text {
    ($text:expr, $color:expr) => {{
        format_args!("\x1b[{}m{}\x1b[0m", $color, $text)
    }};
}

/**
 * 红色 91
 * 绿色 92
 * 黄色 93
 * 蓝色 94
 */

#[macro_export]
macro_rules! error{
    ($info:expr) => {
        println!("{}",color_text!($info,91));
    }
}

#[macro_export]
macro_rules! debug{
    ($info:expr) => {
        println!("{}",color_text!($info,92));
    }
}

#[macro_export]
macro_rules! warn{
    ($info:expr) => {
        println!("{}",color_text!($info,93));
    }
}

#[macro_export]
macro_rules! info{
    ($info:expr) => {
        println!("{}",color_text!($info,94));
    }
}