#[cfg(target_os = "windows")]
extern crate winapi;

#[cfg(target_os = "macos")]
use objc::{msg_send,sel,sel_impl};
#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
use tauri::{Manager, Runtime};
use window_shadows::set_shadow;

pub fn set_window_shadow<R: Runtime>(app: &tauri::App<R>) {
    let window: tauri::Window<R> = app.get_window("skylark").unwrap();
    set_shadow(&window, true).expect("Unsupported platform!");
}

#[cfg(target_os = "macos")]
#[tauri::command(rename_all = "camelCase")]
pub fn set_window_hide_macos() -> String {
    unsafe {
        // 获取 NSApplication 的类
        let ns_app = Class::get("NSApplication").expect("NSApplication class not found");

        // 获取共享应用程序实例
        let app: *mut Object = msg_send![ns_app, sharedApplication];

        // 隐藏应用程序
        let _: () = msg_send![app, hide:app];
    }
    "".to_string()
}

#[cfg(target_os = "macos")]
#[tauri::command(rename_all = "camelCase")]
pub fn set_window_show() -> String {
    unsafe {
        // 获取 NSApplication 的类
        let ns_app = Class::get("NSApplication").expect("NSApplication class not found");

        // 获取共享应用程序实例
        let app: *mut Object = msg_send![ns_app, sharedApplication];

        // 激活应用程序（显示窗口）
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
    "".to_string()
}

#[cfg(target_os = "windows")]
#[tauri::command(rename_all = "camelCase")]
pub fn set_window_show() -> String {

    "".to_string()
}

#[cfg(target_os = "macos")]
pub fn register_global_hotkey() {
    unsafe {
        let ns_app = Class::get("NSApplication").expect("NSApplication class not found");
        let app: *mut Object = msg_send![ns_app, sharedApplication];
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
}



#[cfg(target_os = "windows")]
fn set_cursor_pos(x: i32, y: i32) -> bool {
    use winapi::um::winuser::SetCursorPos;
    let ret = unsafe { SetCursorPos(x, y) };
    ret == 1
}

#[cfg(target_os = "windows")]
fn get_cursor_pos() -> Option<(i32, i32)> {
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::GetCursorPos;

    let mut pt = POINT { x: -1, y: -1 };
    let ret = unsafe { GetCursorPos(&mut pt) };
    if ret != 1 || pt.x == -1 && pt.y == -1 {
        None
    } else {
        Some((pt.x, pt.y))
    }
}