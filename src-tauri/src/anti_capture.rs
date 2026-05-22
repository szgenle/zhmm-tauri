//! 跨平台防截屏工具
//!
//! - macOS: 调用 NSWindow.setSharingType: 设为 NSWindowSharingNone(0)，
//!   屏幕录制 / 截图 / 屏幕共享抓取到的是黑色区域。
//! - Windows: 调用 user32::SetWindowDisplayAffinity，参数
//!   WDA_EXCLUDEFROMCAPTURE(0x11，Win10 2004+) 优先；失败时回退 WDA_MONITOR(0x01)。
//! - 其他平台: 仅记录日志，不可靠。
//!
//! 任何防截屏方案都无法防御摄像头翻拍、外接采集卡等外部途径。

#![allow(dead_code)]

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::c_void;
    use std::os::raw::c_char;

    // NSWindow.sharingType 常量
    const NS_WINDOW_SHARING_NONE: u64 = 0;
    const NS_WINDOW_SHARING_READ_ONLY: u64 = 1;

    #[link(name = "objc", kind = "dylib")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> *const c_void;
    }

    extern "C" {
        fn objc_msgSend();
    }

    /// 给定 NSWindow 指针，应用 / 撤销防截屏保护
    pub fn apply(ns_window: *mut c_void, enabled: bool) -> bool {
        if ns_window.is_null() {
            return false;
        }
        unsafe {
            let sel = sel_registerName(c"setSharingType:".as_ptr() as *const c_char);
            // [ns_window setSharingType:value]
            let msg_send: unsafe extern "C" fn(*mut c_void, *const c_void, u64) =
                std::mem::transmute(objc_msgSend as *const ());
            let value = if enabled {
                NS_WINDOW_SHARING_NONE
            } else {
                NS_WINDOW_SHARING_READ_ONLY
            };
            msg_send(ns_window, sel, value);
        }
        true
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;

    const WDA_NONE: u32 = 0x00000000;
    const WDA_MONITOR: u32 = 0x00000001;
    const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;

    #[link(name = "user32")]
    extern "system" {
        fn SetWindowDisplayAffinity(hwnd: *mut c_void, affinity: u32) -> i32;
    }

    pub fn apply(hwnd: *mut c_void, enabled: bool) -> bool {
        if hwnd.is_null() {
            return false;
        }
        unsafe {
            if enabled {
                // 优先 EXCLUDEFROMCAPTURE，失败回退 MONITOR
                if SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) != 0 {
                    return true;
                }
                SetWindowDisplayAffinity(hwnd, WDA_MONITOR) != 0
            } else {
                SetWindowDisplayAffinity(hwnd, WDA_NONE) != 0
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod imp {
    use std::ffi::c_void;
    pub fn apply(_handle: *mut c_void, _enabled: bool) -> bool {
        false
    }
}

/// 在 macOS 上对 NSWindow 应用 / 撤销防截屏
#[cfg(target_os = "macos")]
pub fn apply_macos(ns_window: *mut std::ffi::c_void, enabled: bool) -> bool {
    imp::apply(ns_window, enabled)
}

/// 在 Windows 上对 HWND 应用 / 撤销防截屏
#[cfg(target_os = "windows")]
pub fn apply_windows(hwnd: *mut std::ffi::c_void, enabled: bool) -> bool {
    imp::apply(hwnd, enabled)
}
