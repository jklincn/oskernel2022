/// # Rust语言相关参数
/// `os/src/lang_items.rs`
/// ## 实践功能
/// - 用于对接panic!宏的panic函数
//

use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]    //通知编译器用panic函数来对接 panic! 宏
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("[kernel] Panicked: {}", info.message().unwrap());
    }
    shutdown()
}