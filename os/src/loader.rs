/// # 程序加载模块
/// `os/src/loader.rs`
/// ## 实现功能
/// ```
/// pub fn get_num_app() -> usize
/// pub fn get_app_data(app_id: usize) -> &'static [u8]
/// ```
/// - 仅实现代码段的读取，解析工作由 `mm::memory_set::MemorySet::from_elf` 实现
//

use alloc::vec::Vec;
use lazy_static::*;

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

lazy_static! {
    /// 全局可见 `只读` 向量，顺序保存了所有应用的名字
    static ref APP_NAMES: Vec<&'static str> = {
        // 根据 link_app.S 的全局标签将应用名字顺序存在 APP_NAMES 向量中
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    end = end.add(1);   // 循环加一找字符串结尾
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}

/// 按照应用的名字来查找获得应用的 ELF 数据
#[allow(unused)]
pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    (0..num_app)
        .find(|&i| APP_NAMES[i] == name)
        .map(get_app_data)
}

/// 打印出所有可用的应用的名字
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in APP_NAMES.iter() {
        println!("{}", app);
    }
    println!("**************/");
}
