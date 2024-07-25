use std::{
    ffi::OsStr,
    os::windows::prelude::OsStrExt,
    ptr::{null, null_mut},
};

use winapi::{
    shared::windef::HWND,
    um::winuser::{
        AddClipboardFormatListener, CreateWindowExW, GetMessageW, HWND_MESSAGE, MSG,
        WM_CLIPBOARDUPDATE,
    },
};

fn main() {
    ClipboardListen::run(move || {
        println!("clipboard updated.");
        //TODO::剪贴板内容获取
    });
}

pub struct ClipboardListen {}

impl ClipboardListen {
    pub fn run<F: Fn() + Send + 'static>(callback: F) {
        std::thread::spawn(move || {
            for msg in Message::new() {
                match msg.message {
                    WM_CLIPBOARDUPDATE => callback(),
                    _ => (),
                }
            }
        });
    }
}

pub struct Message {
    hwnd: HWND,
}

impl Message {
    pub fn new() -> Self {
        // 创建消息窗口
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                str_to_lpcwstr("STATIC").as_ptr(),
                null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                // wnd_class.hInstance,
                null_mut(),
                null_mut(),
            )
        };
        if hwnd == null_mut() {
            panic!("CreateWindowEx failed");
        }

        unsafe { AddClipboardFormatListener(hwnd) };

        Self { hwnd }
    }

    fn get(&self) -> Option<MSG> {
        let mut msg = unsafe { std::mem::zeroed() };
        let ret = unsafe { GetMessageW(&mut msg, self.hwnd, 0, 0) };
        if ret == 1 {
            Some(msg)
        } else {
            None
        }
    }
}

impl Iterator for Message {
    type Item = MSG;

    fn next(&mut self) -> Option<Self::Item> {
        self.get()
    }
}

fn str_to_lpcwstr(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
