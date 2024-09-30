use std::collections::HashMap;
use std::process::Command;
use crate::utils::database::{self, Record};
use crate::config::Config;
use crate::utils::{img_factory, json_factory, string_factory, file_factory};
use anyhow::Result;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use enigo::{Enigo, Key, Keyboard, Settings};
use std::thread;
use arboard::Clipboard;

const CHANGE_DEFAULT_MSG: &str = "ok";

pub struct ClipboardWatcher;

pub struct ClipboardOperator;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct ImageDataDB {
    pub width: usize,
    pub height: usize,
    pub base64: String,
    pub title: String,
}
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct FileDataDB {
    pub file_count: usize,
    pub files: String,
    pub title: String,
}


impl ClipboardOperator {
    pub fn set_text(text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    }

    pub fn get_text() -> Result<String> {
        let mut clipboard = Clipboard::new()?;
        let text = clipboard.get_text()?;
        Ok(text)
    }

    pub fn paste_text(text: &str) -> Result<()> {
        let mut enigo: Enigo = Enigo::new(&Settings::default()).unwrap();
        let _ = enigo.text(text);
        Ok(())
    }

    pub fn set_image(data: ImageDataDB) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        let img_data = img_factory::base64_to_rgba8(&data.base64).unwrap();
        clipboard.set_image(img_data)?;
        Ok(())
    }

    pub fn get_image() -> Result<String> {
        let mut clipboard = Clipboard::new()?;
        let img = clipboard.get_image();
        let data = img.map(|img| {
            let base64 = img_factory::rgba8_to_base64(&img);
            return base64;
        });
        if data.is_err() {
            Ok("".to_string())
        } else {
            Ok(data.unwrap())
        }
    }

    pub fn set_file(file_path: &str) {
        // todo 多文件
        #[cfg(target_os = "windows")]{
            let output = Command::new("powershell").arg("src/utils/clipboard_file_win.ps1").arg(file_path).output().expect("");
            thread::sleep(Duration::milliseconds(500).to_std().unwrap());
            let mut enigo: Enigo = Enigo::new(&Settings::default()).unwrap();
            enigo.key(Key::Control, enigo::Direction::Press).expect("error");
            enigo.key(Key::Unicode('v'), enigo::Direction::Click).expect("error");
            enigo.key(Key::Control, enigo::Direction::Release).expect("error");
        }
        #[cfg(target_os = "macos")]{
            let command_data = format!("'set the clipboard to POSIX file \"'{}'\"'",file_path);
            let _ = Command::new("osascript").arg("-e").arg(command_data).output().expect("");
            thread::sleep(Duration::milliseconds(500).to_std().unwrap());
            let mut enigo: Enigo = Enigo::new(&Settings::default()).unwrap();
            enigo.key(Key::Meta, enigo::Direction::Press).expect("error");
            enigo.key(Key::Unicode('v'), enigo::Direction::Click).expect("error");
            enigo.key(Key::Meta, enigo::Direction::Release).expect("error");
        }
    }

    pub fn get_file() -> Vec<(String, String)> {
        file_factory::get_clipboard_files()
    }
}

#[cfg(target_os = "macos")]
pub fn get_active_application() -> Option<String> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let workspace: *mut Object = msg_send![Class::get("NSWorkspace").unwrap(), sharedWorkspace];
        let active_app: *mut Object = msg_send![workspace, frontmostApplication];

        if active_app.is_null() {
            return None;
        }

        let app_name: *mut Object = msg_send![active_app, localizedName];
        if app_name.is_null() {
            return None;
        }

        let app_name_str: *const libc::c_char = msg_send![app_name, UTF8String];
        Some(std::ffi::CStr::from_ptr(app_name_str).to_string_lossy().into_owned())
    }
}
#[cfg(target_os = "windows")]
// fn get_active_application() -> Option<(String, String)> {
fn get_active_application() -> Option<String> {
    use std::ffi::OsString;
    use std::iter::once;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::psapi::GetModuleBaseNameW;
    use winapi::um::winnt::PROCESS_QUERY_INFORMATION;

    unsafe {
        // 获取前台窗口句柄
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        // 获取窗口标题
        let mut title: [u16; 512] = [0; 512];
        let length = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        let window_title = if length > 0 {
            OsString::from_wide(&title[..length as usize])
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        };

        // 获取进程ID
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);

        // 打开进程
        let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, process_id);
        if process_handle.is_null() {
            // return Some((window_title, String::new()));
            return Some(window_title);
        }

        // 获取可执行文件名
        let mut exe_name: [u16; 512] = [0; 512];
        let length = GetModuleBaseNameW(process_handle, null_mut(), exe_name.as_mut_ptr(), exe_name.len() as u32);
        let exe_name = if length > 0 {
            OsString::from_wide(&exe_name[..length as usize])
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        };

        // 关闭进程句柄
        kernel32::CloseHandle(process_handle);

        // Some((window_title, exe_name))
        Some(window_title)
    }
}


impl ClipboardWatcher {
    pub fn start() {
        tauri::async_runtime::spawn(async {
            // 1000毫秒检测一次剪切板变化
            let wait_millis = 1000i64;
            let mut last_content_md5 = String::new();
            let mut last_img_md5 = String::new();
            let mut clipboard = Clipboard::new().unwrap();
            let limit = Config::new().get_clipboard_record_limit();
            println!("start clipboard watcher");
            loop {
                let mut need_notify = false;
                let db = database::RecordSQL::new();
                let files = file_factory::get_clipboard_files();
                let current_app = get_active_application().unwrap_or("".to_string());
                if !files.is_empty() {
                    let files_string = json_factory::stringify(&files).unwrap();
                    let md5 = string_factory::md5(&files_string);
                    if md5 != last_content_md5 {
                        let files_string = json_factory::stringify(&files).unwrap();
                        println!("获取到新文件: {:?}", files);
                        let content_db = FileDataDB {
                            file_count: files.len(),
                            files: files_string,
                            title: format!("{} File{}: {}", files.len(), if files.len() > 1 { "s" } else { "" }, files[0].0.split("/").last().unwrap()),
                        };
                        let content = json_factory::stringify(&content_db).unwrap();
                        let res = db.insert_if_not_exist(&Record {
                            content: content.clone(),
                            content_preview: Some(content.clone()),
                            data_type: "file".to_string(),
                            source: current_app.clone(),
                            ..Default::default()
                        });
                        match res {
                            Ok(_) => {
                                need_notify = true;
                            }
                            Err(e) => {
                                println!("insert record error: {}", e);
                            }
                        }
                        last_content_md5 = md5.clone();
                    }
                } else {
                    let text = clipboard.get_text();
                    let _ = text.map(|text| {
                        let content_origin = text.clone();
                        let content = text.trim();
                        let md5 = string_factory::md5(&content_origin);
                        if !content.is_empty() && md5 != last_content_md5 {
                            // 说明有新内容
                            println!("获取到新文本: {}", content);
                            let content_preview = if content.len() > 1000 {
                                Some(content.chars().take(1000).collect())
                            } else {
                                Some(content.to_string())
                            };
                            let res = db.insert_if_not_exist(&Record {
                                content: content_origin,
                                content_preview,
                                source: current_app.clone(),
                                ..Default::default()
                            });
                            match res {
                                Ok(_) => {
                                    need_notify = true;
                                }
                                Err(e) => {
                                    println!("insert record error: {}", e);
                                }
                            }
                            last_content_md5 = md5;
                        }
                    });
                }

                let img = clipboard.get_image();
                let _ = img.map(|img| {
                    let img_md5 = string_factory::md5_by_bytes(&img.bytes);
                    let img_size = (img.bytes.len() as f64) / 1024.0;
                    if img_md5 != last_img_md5 {
                        // 有新图片产生
                        println!("获取到新图片md5: {}", img_md5);
                        let base64 = img_factory::rgba8_to_base64(&img);
                        let content_db = ImageDataDB {
                            width: img.width,
                            height: img.height,
                            base64,
                            title: format!("Image:{}×{}({:.2}kb)", img.width, img.height, img_size).to_string(),
                        };
                        // 压缩画质作为预览图，防止渲染时非常卡顿
                        let jpeg_base64 = img_factory::rgba8_to_jpeg_base64(&img, 70);
                        let content_preview_db = ImageDataDB {
                            width: img.width,
                            height: img.height,
                            base64: jpeg_base64,
                            title: format!("Image:{}×{}({:.2}kb)", img.width, img.height, img_size).to_string(),
                        };
                        let content = json_factory::stringify(&content_db).unwrap();
                        let content_preview = json_factory::stringify(&content_preview_db).unwrap();
                        let res = db.insert_if_not_exist(&Record {
                            content,
                            content_preview: Some(content_preview),
                            data_type: "image".to_string(),
                            source: current_app.clone(),
                            ..Default::default()
                        });
                        match res {
                            Ok(_) => {
                                drop(img);
                                need_notify = true;
                            }
                            Err(e) => {
                                println!("insert record error: {}", e);
                            }
                        }
                        last_img_md5 = img_md5;
                    }
                });

                let res = db.delete_over_limit(limit as usize);
                if let Ok(success) = res {
                    if success {
                        need_notify = true;
                    }
                }
                if need_notify {
                    //TODO 显示通知窗口
                    println!("通知一下");
                }
                thread::sleep(Duration::milliseconds(wait_millis).to_std().unwrap());
            }
        });
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_history_all() -> Vec<Record> {
    let db = database::RecordSQL::new();
    db.find_all().unwrap()
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_history_part(limit: i32, offset: i32) -> Vec<Record> {
    let db = database::RecordSQL::new();
    let mut result: Vec<Record> = db.find_part(limit, offset).unwrap();
    let mut app_icon_list: HashMap<String, String> = HashMap::new();
    let db_app = database::IndexSQL::new();
    for mut record in &mut result {
        let icon = app_icon_list.get(&record.source);
        if icon.is_none() {
            let r = db_app.find_app_icon(&record.source).unwrap();
            record.app_icon = r.icon.clone();
            app_icon_list.insert(r.title, r.icon);
        } else {
            record.app_icon = icon.unwrap().clone();
        }
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_history_search(keyword: &str, offset: i32) -> Vec<Record> {
    let db = database::RecordSQL::new();
    db.find_all().unwrap()
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_history_id(id: u64) -> Record {
    let db = database::RecordSQL::new();
    db.find_by_id(id).unwrap()
}

