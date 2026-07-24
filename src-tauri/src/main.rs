// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// 单例锁：防止管理器被同时开启多个实例
/// 通过 Windows 命名互斥量实现，若已有同名互斥量存在则直接退出当前进程
#[cfg(windows)]
fn ensure_single_instance() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    extern "system" {
        fn CreateMutexW(lpMutexAttributes: *mut std::ffi::c_void, bInitialOwner: i32, lpName: *const u16) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }
    const ERROR_ALREADY_EXISTS: u32 = 183;
    let name: Vec<u16> = OsStr::new("TRAEWorkCNManager_SingleInstance_v1")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let handle = CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // 已有另一个管理器实例在运行，静默退出
            std::process::exit(0);
        }
        // 持有互斥量直到进程结束（不主动 Release，进程退出时自动释放）
        std::mem::forget(handle);
    }
}

fn main() {
    #[cfg(windows)]
    ensure_single_instance();
    trae_auto_lib::run()
}
