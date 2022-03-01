use core::panic::PanicInfo;

#[panic_handler]    //通知编译器用panic函数来对接 panic! 宏
fn panic(_info:&PanicInfo) -> ! {
    //panic处理函数的函数签名需要一个 PanicInfo 的不可变借用作为输入参数，它在核心库中得以保留，这也是我们第一次与核心库打交道。
    // 之后我们会从 PanicInfo 解析出错位置并打印出来，然后杀死应用程序。但目前我们什么都不做只是在原地 loop
    loop{}
}