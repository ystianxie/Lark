use cocoa::appkit::NSPasteboard;
use cocoa::base::nil;
use cocoa::foundation::{NSArray, NSAutoreleasePool};
use objc::{msg_send, sel, sel_impl};
use objc::runtime::{Class, Object};

#[cfg(target_os = "macos")]
pub fn get_clipboard_files() -> Vec<String> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let pasteboard = NSPasteboard::generalPasteboard(nil);

        let classes = vec![
            Class::get("NSURL").unwrap() as *const _ as *mut Object,
        ];

        let objects = pasteboard.readObjectsForClasses_options(
            NSArray::arrayWithObjects(nil, &classes),
            nil,
        );

        if objects.is_null() {
            return Vec::new();
        }

        let count = objects.count();
        let mut files = Vec::new();

        for i in 0..count {
            let object = objects.objectAtIndex(i);
            let url: *mut Object = object as *mut Object;

            let path: *mut Object = msg_send![url, path];
            if !path.is_null() {
                let path_str: *const libc::c_char = msg_send![path, UTF8String];
                let path_string = std::ffi::CStr::from_ptr(path_str).to_string_lossy().into_owned();
                files.push(path_string);
            }
        }
        files
    }
}


#[cfg(target_os = "windows")]
fn get_clipboard_files() -> Vec<String> {
    use std::ptr;
    use winapi::um::winuser::{OpenClipboard, CloseClipboard, GetClipboardData, CF_HDROP};
    use winapi::um::shellapi::{DragQueryFileW, DragFinish};
    let mut files = Vec::new();

    unsafe {
        if OpenClipboard(ptr::null_mut()) != 0 {
            let handle = GetClipboardData(CF_HDROP);
            if !handle.is_null() {
                let count = DragQueryFileW(handle, 0xFFFFFFFF, ptr::null_mut(), 0);
                for i in 0..count {
                    let mut buffer: [u16; 260] = [0; 260];
                    let length = DragQueryFileW(handle, i, buffer.as_mut_ptr(), buffer.len() as u32);
                    if length > 0 {
                        let filename = String::from_utf16_lossy(&buffer[..length as usize]);
                        files.push(filename);
                    }
                }
                DragFinish(handle);
            }
            CloseClipboard();
        }
    }

    files
}
