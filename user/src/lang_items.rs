// user/src/lang_items.rs
use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]    //通知编译器用panic函数来对接 panic! 宏
fn panic(info:&PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Panicked: {}", info.message().unwrap());
    }
    shutdown()
}