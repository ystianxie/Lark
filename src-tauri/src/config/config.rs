use std::collections::HashMap;
use std::io::Write;
use serde_json::Value;
use crate::utils::dirs::config_path;
use anyhow::Result;
use applications::prelude::f;
use serde::{Deserialize, Serialize};
use crate::utils::database::Record;

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseConfig {
    app_name: String,
    version: String,
    pub hotkey_awaken: String,
    pub hotkey_clipboard: String,
    clipboard_record_count_switch: bool,
    clipboard_record_count: Option<i32>,
    clipboard_record_text_switch: bool,
    clipboard_record_text_time: Option<i32>,
    clipboard_record_image_switch: bool,
    clipboard_record_image_time: Option<i32>,
    clipboard_record_file_switch: bool,
    clipboard_record_file_time: Option<i32>,
    local_file_search_paths: Vec<String>,
}
impl Default for BaseConfig {
    #[cfg(target_os = "macos")]
    fn default() -> Self {
        Self {
            app_name: "lark".to_string(),
            version: "1.0.0".to_string(),
            hotkey_awaken: "Option+Space".to_string(),
            hotkey_clipboard: "Shift+Meta+V".to_string(),
            clipboard_record_count_switch: false,
            clipboard_record_count: Some(100),
            clipboard_record_text_switch: false,
            clipboard_record_text_time: Some(10),
            clipboard_record_image_switch: false,
            clipboard_record_image_time: Some(5),
            clipboard_record_file_switch: false,
            clipboard_record_file_time: Some(1),
            local_file_search_paths: vec![],
        }
    }
    #[cfg(target_os = "windows")]
    fn default() -> Self {
        Self {
            app_name: "lark".to_string(),
            version: "1.0.0".to_string(),
            hotkey_awaken: "Alt+Space".to_string(),
            hotkey_clipboard: "Shift+Alt+V".to_string(),
            clipboard_record_count_switch: false,
            clipboard_record_count: Some(100),
            clipboard_record_text_switch: false,
            clipboard_record_text_time: Some(10),
            clipboard_record_image_switch: false,
            clipboard_record_image_time: Some(5),
            clipboard_record_file_switch: false,
            clipboard_record_file_time: Some(1),
            local_file_search_paths: vec![],
        }
    }
}
#[derive(Debug)]
enum ConfigUpdate {
    AppName(String),
    Version(String),
    HotkeyAwaken(String),
    HotkeyClipboard(String),
    ClipboardRecordCountSwitch(bool),
    ClipboardRecordCount(Option<i32>),
    ClipboardRecordTextSwitch(bool),
    ClipboardRecordTextTime(Option<i32>),
    ClipboardRecordImageSwitch(bool),
    ClipboardRecordImageTime(Option<i32>),
    ClipboardRecordFileSwitch(bool),
    ClipboardRecordFileTime(Option<i32>),
    LocalFileSearchPaths(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug,Default)]
pub struct ConfigData {
    pub base: BaseConfig,
    plugins: HashMap<String, Value>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    config: ConfigData,
}


impl Config {
    pub fn new() -> Self {
        Self {
            config: Self::read_local_config().unwrap()
        }
    }
    pub fn get_clipboard_record_limit(&self) -> i32 {
        self.config.base.clipboard_record_count.unwrap_or(-1)
    }

    pub fn read_local_config() -> Result<ConfigData> {
        let config_file_path = config_path()?;
        let config = ConfigData { ..Default::default() };
        if !config_file_path.exists(){
            let file_result = std::fs::File::create(config_file_path);
            match file_result {
                Ok(mut file)=>{
                    let config = ConfigData { ..Default::default() };
                    let config_string = serde_json::to_string(&config).unwrap_or("".to_string())  ;
                    file.write_all(&config_string.as_bytes()).expect("写入失败!");
                    Ok(config)
                }
                Err(e)=>{
                    eprintln!("创建失败！");
                    Ok(config)
                }
            }
        }else {
            let file_result = std::fs::File::open(config_file_path)?;
            let config  = serde_json::from_reader(&file_result).unwrap_or(config);
            Ok(config)
        }
    }

    pub fn update_local_config(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::AppName(value) => self.config.base.app_name = value,
            ConfigUpdate::Version(value) => self.config.base.version = value,
            ConfigUpdate::HotkeyAwaken(value) => self.config.base.hotkey_awaken = value,
            ConfigUpdate::HotkeyClipboard(value) => self.config.base.hotkey_clipboard = value,
            ConfigUpdate::ClipboardRecordCountSwitch(value) => self.config.base.clipboard_record_count_switch = value,
            ConfigUpdate::ClipboardRecordCount(value) => self.config.base.clipboard_record_count = value,
            ConfigUpdate::ClipboardRecordTextSwitch(value) => self.config.base.clipboard_record_text_switch = value,
            ConfigUpdate::ClipboardRecordTextTime(value) => self.config.base.clipboard_record_text_time = value,
            ConfigUpdate::ClipboardRecordImageSwitch(value) => self.config.base.clipboard_record_image_switch = value,
            ConfigUpdate::ClipboardRecordImageTime(value) => self.config.base.clipboard_record_image_time = value,
            ConfigUpdate::ClipboardRecordFileSwitch(value) => self.config.base.clipboard_record_file_switch = value,
            ConfigUpdate::ClipboardRecordFileTime(value) => self.config.base.clipboard_record_file_time = value,
            ConfigUpdate::LocalFileSearchPaths(value) => self.config.base.local_file_search_paths = value
        }
    }
    pub fn save_local_config(&self) -> Result<()> {
        let mut file = std::fs::File::create(config_path()?)?;
        let config = serde_json::to_string(&self.config).unwrap_or("".to_string());
        file.write_all(config.as_bytes()).expect("写入到文件失败！");
        Ok(())
    }
    pub fn register_plugin_config(&mut self, plugin_name: &str, config: Value) {
        self.config.plugins.insert(plugin_name.to_string(), config);
    }
}
#[tauri::command(rename_all = "camelCase")]
pub fn save_setting(setting_info: Value) {
    // todo 保存设置结果到数据库
    println!("Received JSON data: {}", setting_info);
}