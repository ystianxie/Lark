use crate::utils::database::{self, Record};
use crate::config::Config;
use crate::utils::{img_factory, json_factory, string_factory};
use anyhow::Result;
use arboard::Clipboard;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use enigo::{Enigo, Key, Keyboard, Settings};

use std::thread;
const CHANGE_DEFAULT_MSG: &str = "ok";

pub struct ClipboardWatcher;

pub struct ClipboardOperator;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct ImageDataDB {
    pub width: usize,
    pub height: usize,
    pub base64: String,
}

impl ClipboardOperator {
    pub fn set_text(text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    }

    pub fn set_image(data: ImageDataDB) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        let img_data = img_factory::base64_to_rgba8(&data.base64).unwrap();
        clipboard.set_image(img_data)?;
        Ok(())
    }

    pub fn get_text() -> Result<String> {
        let mut clipboard = Clipboard::new()?;
        let text = clipboard.get_text()?;
        Ok(text)
    }

    pub fn paste_text(text:&str) -> Result<()> {
        let mut enigo: Enigo = Enigo::new(&Settings::default()).unwrap();
        let _ = enigo.text(text);
        Ok(())
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
            println!("start clipboard watcher");
            loop {
                let mut need_notify = false;
                let db = database::SqliteDB::new();
                let text = clipboard.get_text();
                let _ = text.map(|text| {
                    let content_origin = text.clone();
                    let content = text.trim();
                    let md5 = string_factory::md5(&content_origin);
                    if !content.is_empty() && md5 != last_content_md5 {
                        // 说明有新内容
                        let content_preview = if content.len() > 1000 {
                            Some(content.chars().take(1000).collect())
                        } else {
                            Some(content.to_string())
                        };
                        let res = db.insert_if_not_exist(&Record {
                            content: content_origin,
                            content_preview,
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

                let img = clipboard.get_image();
                let _ = img.map(|img| {
                    let img_md5 = string_factory::md5_by_bytes(&img.bytes);
                    if img_md5 != last_img_md5 {
                        // 有新图片产生
                        let base64 = img_factory::rgba8_to_base64(&img);
                        let content_db = ImageDataDB {
                            width: img.width,
                            height: img.height,
                            base64,
                        };
                        // 压缩画质作为预览图，防止渲染时非常卡顿
                        let jpeg_base64 = img_factory::rgba8_to_jpeg_base64(&img, 75);
                        let content_preview_db = ImageDataDB {
                            width: img.width,
                            height: img.height,
                            base64: jpeg_base64,
                        };
                        let content = json_factory::stringfy(&content_db).unwrap();
                        let content_preview = json_factory::stringfy(&content_preview_db).unwrap();
                        let res = db.insert_if_not_exist(&Record {
                            content,
                            content_preview: Some(content_preview),
                            data_type: "image".to_string(),
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
                // TODO 获取剪贴板历史保存数量
                let limit = Config::get_clipboard_record_limit().clone();
                // if let Some(l) = limit {
                //     let res = db.delete_over_limit(l as usize);
                //     if let Ok(success) = res {
                //         if success {
                //             need_notify = true;
                //         }
                //     }
                // }
                let res = db.delete_over_limit(limit as usize);
                if let Ok(success) = res {
                    if success {
                        need_notify = true;
                    }
                }
                if need_notify {
                //TODO 显示通知窗口
                }
                thread::sleep(Duration::milliseconds(wait_millis).to_std().unwrap());
            }
        });
    }
}
