use crate::config::{ClipboardRetention, Config};
use crate::utils::database::{self, Record};
#[cfg(target_os = "windows")]
use crate::utils::icons;
use crate::utils::{file_factory, img_factory, json_factory, string_factory};
use anyhow::Result;
use arboard::Clipboard;
use chrono::Duration;
use enigo::{Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration as StdDuration, Instant};

pub struct ClipboardWatcher;

pub struct ClipboardOperator;

fn apply_clipboard_retention(db: &database::RecordSQL, retention: ClipboardRetention) -> bool {
    let mut changed = false;
    if let Some(limit) = retention.count {
        match db.delete_over_limit(limit) {
            Ok(removed) => changed |= removed,
            Err(error) => eprintln!("清理超量剪贴板记录失败: {error}"),
        }
    }
    for (data_type, days) in [
        ("text", retention.text_days),
        ("image", retention.image_days),
        ("file", retention.file_days),
    ] {
        if let Some(days) = days {
            match db.delete_expired(data_type, days) {
                Ok(removed) => changed |= removed,
                Err(error) => eprintln!("清理过期{data_type}剪贴板记录失败: {error}"),
            }
        }
    }
    changed
}

#[derive(Default, Clone)]
struct ActiveApplication {
    name: String,
    path: String,
}

// 文本结果粘贴会先写系统剪贴板，而剪贴板监听器也会读取它。
// 记录一次短时的“由 Lark 主动写入”的文本，避免这次内部写入回流到历史库。
struct InternalClipboardText {
    text: String,
    marked_at: Instant,
    sequence: u32,
}

static INTERNAL_CLIPBOARD_TEXT: OnceLock<Mutex<Option<InternalClipboardText>>> = OnceLock::new();
static INTERNAL_CLIPBOARD_BINARY: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();
static INTERNAL_CLIPBOARD_SEQUENCE: OnceLock<Mutex<Option<(u32, Instant)>>> = OnceLock::new();
const INTERNAL_CLIPBOARD_TTL: StdDuration = StdDuration::from_secs(3);

fn set_internal_clipboard_text(text: &str) -> Result<()> {
    let state = INTERNAL_CLIPBOARD_TEXT.get_or_init(|| Mutex::new(None));
    let mut pending = state
        .lock()
        .map_err(|_| anyhow::anyhow!("internal clipboard state is poisoned"))?;
    *pending = None;
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    #[cfg(target_os = "windows")]
    let sequence = get_clipboard_sequence_number();
    #[cfg(not(target_os = "windows"))]
    let sequence = 0;
    *pending = Some(InternalClipboardText {
        text: text.to_string(),
        marked_at: Instant::now(),
        sequence,
    });
    Ok(())
}

fn consume_internal_clipboard_text(text: &str) -> Option<u32> {
    let Some(state) = INTERNAL_CLIPBOARD_TEXT.get() else {
        return None;
    };
    let Ok(mut pending) = state.lock() else {
        return None;
    };
    let Some(marked) = pending.as_ref() else {
        return None;
    };
    if marked.marked_at.elapsed() > INTERNAL_CLIPBOARD_TTL {
        *pending = None;
        return None;
    }
    if marked.text == text {
        return pending.take().map(|marked| marked.sequence);
    }
    None
}

fn mark_internal_clipboard_binary(digest: String) {
    let state = INTERNAL_CLIPBOARD_BINARY.get_or_init(|| Mutex::new(None));
    if let Ok(mut pending) = state.lock() {
        *pending = Some((digest, Instant::now()));
    }
}

fn consume_internal_clipboard_binary(digest: &str) -> bool {
    let Some(state) = INTERNAL_CLIPBOARD_BINARY.get() else {
        return false;
    };
    let Ok(mut pending) = state.lock() else {
        return false;
    };
    let Some((marked_digest, marked_at)) = pending.as_ref() else {
        return false;
    };
    if marked_at.elapsed() > INTERNAL_CLIPBOARD_TTL {
        *pending = None;
        return false;
    }
    if marked_digest == digest {
        *pending = None;
        return true;
    }
    false
}

fn clear_internal_clipboard_binary() {
    if let Some(state) = INTERNAL_CLIPBOARD_BINARY.get() {
        if let Ok(mut pending) = state.lock() {
            *pending = None;
        }
    }
}

#[cfg(target_os = "windows")]
fn mark_internal_clipboard_sequence() {
    let state = INTERNAL_CLIPBOARD_SEQUENCE.get_or_init(|| Mutex::new(None));
    if let Ok(mut pending) = state.lock() {
        *pending = Some((get_clipboard_sequence_number(), Instant::now()));
    }
}

#[cfg(target_os = "windows")]
fn consume_internal_clipboard_sequence(sequence: u32) -> bool {
    let Some(state) = INTERNAL_CLIPBOARD_SEQUENCE.get() else {
        return false;
    };
    let Ok(mut pending) = state.lock() else {
        return false;
    };
    let Some((marked_sequence, marked_at)) = pending.as_ref() else {
        return false;
    };
    if marked_at.elapsed() > INTERNAL_CLIPBOARD_TTL {
        *pending = None;
        return false;
    }
    if *marked_sequence == sequence {
        *pending = None;
        return true;
    }
    false
}

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

    pub fn set_text_for_paste(text: &str) -> Result<()> {
        set_internal_clipboard_text(text)
    }

    pub fn get_text() -> Result<String> {
        let mut clipboard = Clipboard::new()?;
        let text = clipboard.get_text()?;
        Ok(text)
    }

    pub fn paste_text() -> Result<()> {
        // 窗口隐藏后，系统需要短暂时间把焦点还给唤醒前的窗口。
        thread::sleep(Duration::milliseconds(50).to_std()?);

        let mut enigo: Enigo = Enigo::new(&Settings::default())?;
        // 文本已经由调用方写入系统剪贴板，这里只发送一次粘贴快捷键。
        // enigo.text(...) 会逐字符模拟输入，长文本会明显变慢，且容易被目标应用截断。
        #[cfg(target_os = "macos")]
        let modifier = Key::Meta;
        #[cfg(not(target_os = "macos"))]
        let modifier = Key::Control;

        enigo.key(modifier, enigo::Direction::Press)?;
        let paste_result = enigo.key(Key::Unicode('v'), enigo::Direction::Click);
        let release_result = enigo.key(modifier, enigo::Direction::Release);
        paste_result?;
        release_result?;
        Ok(())
    }

    pub fn set_image(data: ImageDataDB) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        let img_data = img_factory::base64_to_rgba8(&data.base64)?;
        let digest = string_factory::md5_by_bytes(&img_data.bytes);
        clipboard.set_image(img_data)?;
        mark_internal_clipboard_binary(digest);
        #[cfg(target_os = "windows")]
        mark_internal_clipboard_sequence();
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
            Ok(data?)
        }
    }

    pub fn set_file(files: &[(String, String)]) -> Result<()> {
        if files.is_empty() {
            return Err(anyhow::anyhow!("没有可粘贴的文件路径"));
        }
        let file_paths: Vec<String> = files.iter().map(|(path, _)| path.clone()).collect();
        let digest = string_factory::md5(&serde_json::to_string(files)?);
        #[cfg(target_os = "windows")]
        {
            let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/utils/clipboard_file_win.ps1");
            let output = Command::new("powershell")
                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                .arg(script)
                .args(file_paths)
                .output()?;
            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "写入文件剪贴板失败: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            mark_internal_clipboard_binary(digest);
            mark_internal_clipboard_sequence();
            return Ok(());
        }
        #[cfg(target_os = "macos")]
        {
            let file_list = file_paths
                .iter()
                .map(|path| format!("POSIX file \"{}\"", path.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(", ");
            let output = Command::new("osascript")
                .arg("-e")
                .arg(format!("set the clipboard to {{{file_list}}}"))
                .output()?;
            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "写入文件剪贴板失败: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            mark_internal_clipboard_binary(digest);
            return Ok(());
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        Err(anyhow::anyhow!("当前平台暂不支持文件剪贴板"))
    }

    pub fn get_file() -> Vec<(String, String)> {
        file_factory::get_clipboard_files()
    }
}

#[cfg(target_os = "macos")]
fn get_active_application() -> Option<ActiveApplication> {
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
        Some(ActiveApplication {
            name: std::ffi::CStr::from_ptr(app_name_str)
                .to_string_lossy()
                .into_owned(),
            path: String::new(),
        })
    }
}
#[cfg(target_os = "windows")]
fn get_active_application() -> Option<ActiveApplication> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

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
        let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process_handle.is_null() {
            return Some(ActiveApplication {
                name: window_title,
                path: String::new(),
            });
        }

        // 获取完整可执行文件路径，作为稳定的应用身份。
        let mut exe_path: [u16; 32768] = [0; 32768];
        let mut length = exe_path.len() as u32;
        let success = kernel32::QueryFullProcessImageNameW(
            process_handle,
            0,
            exe_path.as_mut_ptr(),
            &mut length,
        );
        let executable_path = if success != 0 && length > 0 {
            OsString::from_wide(&exe_path[..length as usize])
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        };

        // 关闭进程句柄
        kernel32::CloseHandle(process_handle);

        let name = Path::new(&executable_path)
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(&window_title)
            .to_string();

        Some(ActiveApplication {
            name,
            path: executable_path,
        })
    }
}

#[cfg(target_os = "windows")]
fn get_clipboard_sequence_number() -> u32 {
    use winapi::um::winuser::GetClipboardSequenceNumber;

    unsafe { GetClipboardSequenceNumber() }
}

impl ClipboardWatcher {
    pub fn start() {
        tauri::async_runtime::spawn(async {
            // 1000毫秒检测一次剪切板变化
            let wait_millis = 1000i64;
            let mut last_content_md5 = String::new();
            let mut last_img_md5 = String::new();
            #[cfg(target_os = "windows")]
            let mut last_clipboard_sequence = 0;
            let mut clipboard = Clipboard::new().unwrap();
            let mut retention = Config::new().clipboard_retention();
            let mut retention_refresh_tick = 0u8;
            println!("start clipboard watcher");
            loop {
                let refresh_retention = retention_refresh_tick == 0;
                if refresh_retention {
                    retention = Config::new().clipboard_retention();
                }
                retention_refresh_tick = (retention_refresh_tick + 1) % 5;
                #[cfg(target_os = "windows")]
                let current_clipboard_sequence = get_clipboard_sequence_number();
                #[cfg(target_os = "windows")]
                let clipboard_changed = current_clipboard_sequence != 0
                    && current_clipboard_sequence != last_clipboard_sequence;
                #[cfg(target_os = "windows")]
                let internal_clipboard_change =
                    consume_internal_clipboard_sequence(current_clipboard_sequence);
                #[cfg(target_os = "windows")]
                if internal_clipboard_change {
                    clear_internal_clipboard_binary();
                }
                #[cfg(not(target_os = "windows"))]
                let clipboard_changed = false;
                #[cfg(not(target_os = "windows"))]
                let internal_clipboard_change = false;
                #[cfg(target_os = "windows")]
                let mut clipboard_snapshot_read = false;
                #[cfg(target_os = "windows")]
                let mut internal_sequence = None;
                let mut need_notify = false;
                let db = database::RecordSQL::new();
                let files = file_factory::get_clipboard_files();
                let current_app = get_active_application().unwrap_or_default();
                if !files.is_empty() {
                    #[cfg(target_os = "windows")]
                    {
                        clipboard_snapshot_read = true;
                    }
                    let files_string = json_factory::stringify(&files).unwrap();
                    let md5 = string_factory::md5(&files_string);
                    if internal_clipboard_change {
                        last_content_md5 = md5.clone();
                    }
                    if !internal_clipboard_change && (clipboard_changed || md5 != last_content_md5)
                    {
                        if consume_internal_clipboard_binary(&md5) {
                            last_content_md5 = md5;
                        } else {
                            let files_string = json_factory::stringify(&files).unwrap();
                            println!("获取到新文件: {:?}", files);
                            let content_db = FileDataDB {
                                file_count: files.len(),
                                files: files_string,
                                title: format!(
                                    "{} File{}: {}",
                                    files.len(),
                                    if files.len() > 1 { "s" } else { "" },
                                    files[0].0.split("/").last().unwrap()
                                ),
                            };
                            let content = json_factory::stringify(&content_db).unwrap();
                            let res = db.insert_if_not_exist(&Record {
                                content: content.clone(),
                                content_preview: Some(content.clone()),
                                data_type: "file".to_string(),
                                source: current_app.name.clone(),
                                source_path: current_app.path.clone(),
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
                    }
                } else {
                    let text = clipboard.get_text();
                    let _ = text.map(|text| {
                        #[cfg(target_os = "windows")]
                        {
                            clipboard_snapshot_read = true;
                        }
                        let content_origin = text.clone();
                        let content = text.trim();
                        let md5 = string_factory::md5(&content_origin);
                        if !internal_clipboard_change
                            && !content.is_empty()
                            && (clipboard_changed || md5 != last_content_md5)
                        {
                            if let Some(sequence) = consume_internal_clipboard_text(&content_origin)
                            {
                                #[cfg(target_os = "windows")]
                                {
                                    internal_sequence = Some(sequence);
                                }
                                // 仍然推进去重游标，避免下一轮轮询再次落库。
                                last_content_md5 = md5;
                            } else {
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
                                    source: current_app.name.clone(),
                                    source_path: current_app.path.clone(),
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
                        }
                    });
                }

                let img = clipboard.get_image();
                let _ = img.map(|img| {
                    #[cfg(target_os = "windows")]
                    {
                        clipboard_snapshot_read = true;
                    }
                    let img_md5 = string_factory::md5_by_bytes(&img.bytes);
                    let img_size = (img.bytes.len() as f64) / 1024.0;
                    if !internal_clipboard_change && (clipboard_changed || img_md5 != last_img_md5)
                    {
                        if consume_internal_clipboard_binary(&img_md5) {
                            last_img_md5 = img_md5;
                            return;
                        }
                        // 有新图片产生
                        println!("获取到新图片md5: {}", img_md5);
                        let base64 = img_factory::rgba8_to_base64(&img);
                        let content_db = ImageDataDB {
                            width: img.width,
                            height: img.height,
                            base64,
                            title: format!("Image:{}×{}({:.2}kb)", img.width, img.height, img_size)
                                .to_string(),
                        };
                        // 压缩画质作为预览图，防止渲染时非常卡顿
                        let jpeg_base64 = img_factory::rgba8_to_jpeg_base64(&img, 70);
                        println!("获取到新图片");
                        let content_preview_db = ImageDataDB {
                            width: img.width,
                            height: img.height,
                            base64: jpeg_base64,
                            title: format!("Image:{}×{}({:.2}kb)", img.width, img.height, img_size)
                                .to_string(),
                        };
                        let content = json_factory::stringify(&content_db).unwrap();
                        let content_preview = json_factory::stringify(&content_preview_db).unwrap();
                        let res = db.insert_if_not_exist(&Record {
                            content,
                            content_preview: Some(content_preview),
                            data_type: "image".to_string(),
                            source: current_app.name.clone(),
                            source_path: current_app.path.clone(),
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

                if (need_notify || refresh_retention) && apply_clipboard_retention(&db, retention) {
                    need_notify = true;
                }
                if need_notify {
                    //TODO 显示通知窗口
                    println!("通知一下");
                }
                #[cfg(target_os = "windows")]
                {
                    if let Some(sequence) = internal_sequence {
                        last_clipboard_sequence = sequence;
                    } else if clipboard_snapshot_read
                        && get_clipboard_sequence_number() == current_clipboard_sequence
                    {
                        last_clipboard_sequence = current_clipboard_sequence;
                    }
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
    for record in &mut result {
        let cache_key = if record.source_path.is_empty() {
            record.source.to_lowercase()
        } else {
            record.source_path.to_lowercase()
        };
        let icon = app_icon_list.get(&cache_key);
        if icon.is_none() {
            let r = db_app
                .find_app_icon(&record.source, &record.source_path)
                .unwrap();
            let mut resolved_icon = r.icon;
            #[cfg(target_os = "windows")]
            if resolved_icon.is_empty() && !record.source_path.is_empty() {
                resolved_icon = icons::get_icon(&record.source_path, 128)
                    .map(base64::encode)
                    .unwrap_or_default();
            }
            record.app_icon = resolved_icon.clone();
            app_icon_list.insert(cache_key, resolved_icon);
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
