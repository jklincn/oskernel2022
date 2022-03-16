/// # 程序加载模块
/// `os/src/loader.rs`
/// ## 实现功能
/// ```
/// pub fn get_num_app() -> usize
/// pub fn get_app_data(app_id: usize) -> &'static [u8]
/// ```
/// - 仅实现代码段的读取，解析工作由 `mm::memory_set::MemorySet::from_elf` 实现
//

/// ### 从 `link_app.S` 根据 `_num_app` 标签取出待加载应用数量
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// ### 读取第 `app_id` 个应用的 ELF 格式可执行文件数据，返回一个切片
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    // 读取应用存放内存地址数组
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}
