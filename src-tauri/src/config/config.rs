use crate::utils::database::Record;
use crate::utils::dirs::config_path;
use anyhow::Result;
use applications::prelude::f;
use serde::{Deserialize, Serialize};
use serde_json::{to_string, Value};
use std::collections::HashMap;
use std::io::Write;
use tauri::Manager;
use walkdir::DirEntry;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextSnippet {
    pub keyword: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaseConfig {
    app_name: String,
    version: String,
    pub hotkey_awaken: String,
    pub hotkey_clipboard: String,
    #[serde(default = "default_hotkey_file_jump")]
    pub hotkey_file_jump: String,
    #[serde(default)]
    clipboard_settings_initialized: bool,
    clipboard_record_count_switch: bool,
    clipboard_record_count: Option<i32>,
    clipboard_record_text_switch: bool,
    clipboard_record_text_time: Option<i32>,
    clipboard_record_image_switch: bool,
    clipboard_record_image_time: Option<i32>,
    clipboard_record_file_switch: bool,
    clipboard_record_file_time: Option<i32>,
    /// None = 尚未初始化；Some([]) = 用户明确不包含任何目录。
    #[serde(default)]
    pub local_file_search_paths: Option<Vec<String>>,
    pub local_file_search_exclude_paths: Vec<String>,
    pub local_file_search_exclude_types: Vec<String>,
    #[serde(default)]
    pub local_app_search_paths: Vec<String>,
    #[serde(default)]
    pub local_app_search_exclude_paths: Vec<String>,
    #[serde(default)]
    pub app_index_initialized: bool,
    #[serde(default)]
    pub file_index_initialized: bool,
    #[serde(default)]
    pub snippets_enabled: bool,
    #[serde(default = "default_snippet_trigger")]
    pub snippet_trigger: String,
    #[serde(default)]
    pub text_snippets: Vec<TextSnippet>,
    /// 全局 Python 解释器路径（可直接指向虚拟环境里的可执行文件）。None = 使用平台默认命令。
    #[serde(default)]
    pub python_interpreter: Option<String>,
}

fn default_snippet_trigger() -> String {
    ";".to_string()
}

fn default_hotkey_file_jump() -> String {
    "Ctrl+G".to_string()
}
impl Default for BaseConfig {
    #[cfg(target_os = "macos")]
    fn default() -> Self {
        Self {
            app_name: "lark".to_string(),
            version: "1.0.0".to_string(),
            hotkey_awaken: "Option+Space".to_string(),
            hotkey_clipboard: "Shift+Meta+V".to_string(),
            hotkey_file_jump: default_hotkey_file_jump(),
            clipboard_settings_initialized: true,
            clipboard_record_count_switch: true,
            clipboard_record_count: Some(100),
            clipboard_record_text_switch: false,
            clipboard_record_text_time: Some(10),
            clipboard_record_image_switch: false,
            clipboard_record_image_time: Some(5),
            clipboard_record_file_switch: false,
            clipboard_record_file_time: Some(1),
            local_file_search_paths: None,
            local_file_search_exclude_paths: vec![
                "/Library".to_string(),
                "/System".to_string(),
                "/private".to_string(),
                "/usr".to_string(),
                "/etc".to_string(),
                "/cores".to_string(),
                "/Volumes".to_string(),
                "/dev".to_string(),
                "~/Library".to_string(),
                "*/node_modules".to_string(),
                "*/src-tauri/target".to_string(),
                "*/venv".to_string(),
                "*/dist".to_string(),
            ],
            local_file_search_exclude_types: vec![
                "plist".to_string(),
                "dylib".to_string(),
                "kext".to_string(),
                "framework".to_string(),
                "app".to_string(),
                "ds_store".to_string(),
                "crash".to_string(),
                "sparseimage".to_string(),
                "kernel".to_string(),
                "xpc".to_string(),
            ],
            local_app_search_paths: Vec::new(),
            local_app_search_exclude_paths: Vec::new(),
            app_index_initialized: false,
            file_index_initialized: false,
            snippets_enabled: false,
            snippet_trigger: default_snippet_trigger(),
            text_snippets: Vec::new(),
            python_interpreter: None,
        }
    }
    #[cfg(target_os = "windows")]
    fn default() -> Self {
        Self {
            app_name: "lark".to_string(),
            version: "1.0.0".to_string(),
            hotkey_awaken: "Alt+Space".to_string(),
            hotkey_clipboard: "Shift+Alt+V".to_string(),
            hotkey_file_jump: default_hotkey_file_jump(),
            clipboard_settings_initialized: true,
            clipboard_record_count_switch: true,
            clipboard_record_count: Some(100),
            clipboard_record_text_switch: false,
            clipboard_record_text_time: Some(10),
            clipboard_record_image_switch: false,
            clipboard_record_image_time: Some(5),
            clipboard_record_file_switch: false,
            clipboard_record_file_time: Some(1),
            local_file_search_paths: None,
            local_file_search_exclude_paths: vec![
                r"C:\Windows".to_string(),
                r"C:\ProgramData".to_string(),
                "C:$Recycle.Bin".to_string(),
                r"C:\Users\Public".to_string(),
                r"C:\Recovery".to_string(),
                r"C:\Program Files\Common Files".to_string(),
                r"C:\Windows\SoftwareDistribution".to_string(),
                r"C:\Windows\Prefetch".to_string(),
                // User/application caches and local development artifacts.
                "*/AppData".to_string(),
                "*/.git".to_string(),
                "*/.hg".to_string(),
                "*/.svn".to_string(),
                "*/node_modules".to_string(),
                "*/target".to_string(),
                "*/build".to_string(),
                "*/venv".to_string(),
                "*/.venv".to_string(),
                "*/env".to_string(),
                "*/.env".to_string(),
                "*/__pycache__".to_string(),
                "*/.pytest_cache".to_string(),
                "*/.mypy_cache".to_string(),
                "*/.ruff_cache".to_string(),
                "*/dist".to_string(),
                "*/out".to_string(),
                "*/coverage".to_string(),
                "*/.cache".to_string(),
                "*/.turbo".to_string(),
                "*/.next".to_string(),
                "*/.nuxt".to_string(),
                "*/vendor".to_string(),
            ],
            local_file_search_exclude_types: vec![
                "sys".to_string(),
                "dll".to_string(),
                "inf".to_string(),
                "dat".to_string(),
                "log".to_string(),
                "bak".to_string(),
                "cab".to_string(),
                "vxd".to_string(),
                "msi".to_string(),
                "evt".to_string(),
            ],
            local_app_search_paths: vec![
                r"D:\App".to_string(),
                r"D:\Apps".to_string(),
                r"C:\App".to_string(),
                r"C:\Apps".to_string(),
            ],
            local_app_search_exclude_paths: Vec::new(),
            app_index_initialized: false,
            file_index_initialized: false,
            snippets_enabled: false,
            snippet_trigger: default_snippet_trigger(),
            text_snippets: Vec::new(),
            python_interpreter: None,
        }
    }
}
#[derive(Debug)]
enum ConfigUpdate {
    AppName(String),
    Version(String),
    HotkeyAwaken(String),
    HotkeyClipboard(String),
    HotkeyFileJump(String),
    ClipboardRecordCountSwitch(bool),
    ClipboardRecordCount(Option<i32>),
    ClipboardRecordTextSwitch(bool),
    ClipboardRecordTextTime(Option<i32>),
    ClipboardRecordImageSwitch(bool),
    ClipboardRecordImageTime(Option<i32>),
    ClipboardRecordFileSwitch(bool),
    ClipboardRecordFileTime(Option<i32>),
    LocalFileSearchPaths(Option<Vec<String>>),
    LocalFileSearchExcludePaths(Vec<String>),
    LocalFileSearchExcludeTypes(Vec<String>),
    LocalAppSearchPaths(Vec<String>),
    LocalAppSearchExcludePaths(Vec<String>),
    PythonInterpreter(Option<String>),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConfigData {
    pub base: BaseConfig,
    #[serde(default)]
    plugins: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    config: ConfigData,
}

#[derive(Debug, Clone, Copy)]
pub struct ClipboardRetention {
    pub count: Option<usize>,
    pub text_days: Option<i32>,
    pub image_days: Option<i32>,
    pub file_days: Option<i32>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            config: Self::read_local_config().unwrap(),
        }
    }
    // pub fn get_file_search_exclude_paths(&self) -> Vec<String> {
    //     let mut paths = vec![];
    //     let home_dir = Manager::path(&self).home_dir().unwrap().to_str().unwrap().to_string();
    //     for mut path in self.config.base.local_file_search_exclude_paths.clone() {
    //         if path.starts_with("~/") {
    //             path = path.replace("~", &home_dir);
    //         }
    //         paths.push(path)
    //     }
    //     paths
    // }
    // todo 设置文件搜索排除 目录和类型

    pub fn read_local_config() -> Result<ConfigData> {
        let config_file_path = config_path()?;
        let config = ConfigData {
            ..Default::default()
        };
        if !config_file_path.exists() {
            let file_result = std::fs::File::create(config_file_path);
            match file_result {
                Ok(mut file) => {
                    let config = ConfigData {
                        ..Default::default()
                    };
                    let config_string =
                        serde_json::to_string_pretty(&config).unwrap_or("".to_string());
                    file.write_all(&config_string.as_bytes())
                        .expect("写入失败!");
                    Ok(config)
                }
                Err(e) => {
                    eprintln!("创建失败！");
                    Ok(config)
                }
            }
        } else {
            let file_result = std::fs::File::open(config_file_path)?;
            let mut config: ConfigData = serde_json::from_reader(&file_result).unwrap_or(config);
            if !config.base.clipboard_settings_initialized {
                // Older builds always enforced the 100-item limit even though the
                // persisted switch was unused. Preserve that effective behavior.
                config.base.clipboard_settings_initialized = true;
                config.base.clipboard_record_count_switch = true;
            }
            // Keep user customizations, including intentionally removed exclusions.
            Ok(config)
        }
    }

    pub fn update_local_config(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::AppName(value) => self.config.base.app_name = value,
            ConfigUpdate::Version(value) => self.config.base.version = value,
            ConfigUpdate::HotkeyAwaken(value) => self.config.base.hotkey_awaken = value,
            ConfigUpdate::HotkeyClipboard(value) => self.config.base.hotkey_clipboard = value,
            ConfigUpdate::HotkeyFileJump(value) => self.config.base.hotkey_file_jump = value,
            ConfigUpdate::ClipboardRecordCountSwitch(value) => {
                self.config.base.clipboard_record_count_switch = value
            }
            ConfigUpdate::ClipboardRecordCount(value) => {
                self.config.base.clipboard_record_count = value
            }
            ConfigUpdate::ClipboardRecordTextSwitch(value) => {
                self.config.base.clipboard_record_text_switch = value
            }
            ConfigUpdate::ClipboardRecordTextTime(value) => {
                self.config.base.clipboard_record_text_time = value
            }
            ConfigUpdate::ClipboardRecordImageSwitch(value) => {
                self.config.base.clipboard_record_image_switch = value
            }
            ConfigUpdate::ClipboardRecordImageTime(value) => {
                self.config.base.clipboard_record_image_time = value
            }
            ConfigUpdate::ClipboardRecordFileSwitch(value) => {
                self.config.base.clipboard_record_file_switch = value
            }
            ConfigUpdate::ClipboardRecordFileTime(value) => {
                self.config.base.clipboard_record_file_time = value
            }
            ConfigUpdate::LocalFileSearchPaths(value) => {
                self.config.base.local_file_search_paths = value
            }
            ConfigUpdate::LocalFileSearchExcludePaths(value) => {
                self.config.base.local_file_search_exclude_paths = value
            }
            ConfigUpdate::LocalFileSearchExcludeTypes(value) => {
                self.config.base.local_file_search_exclude_types = value
            }
            ConfigUpdate::LocalAppSearchPaths(value) => {
                self.config.base.local_app_search_paths = value
            }
            ConfigUpdate::LocalAppSearchExcludePaths(value) => {
                self.config.base.local_app_search_exclude_paths = value
            }
            ConfigUpdate::PythonInterpreter(value) => self.config.base.python_interpreter = value,
        }
    }
    pub fn clipboard_retention(&self) -> ClipboardRetention {
        let base = &self.config.base;
        ClipboardRetention {
            count: base
                .clipboard_record_count_switch
                .then_some(base.clipboard_record_count)
                .flatten()
                .filter(|value| *value > 0)
                .map(|value| value as usize),
            text_days: base
                .clipboard_record_text_switch
                .then_some(base.clipboard_record_text_time)
                .flatten()
                .filter(|value| *value > 0),
            image_days: base
                .clipboard_record_image_switch
                .then_some(base.clipboard_record_image_time)
                .flatten()
                .filter(|value| *value > 0),
            file_days: base
                .clipboard_record_file_switch
                .then_some(base.clipboard_record_file_time)
                .flatten()
                .filter(|value| *value > 0),
        }
    }
    pub fn save_local_config(&self) -> Result<()> {
        let path = config_path()?;
        let temp_path = path.with_extension("json.tmp");
        let backup_path = path.with_extension("json.bak");
        let mut file = std::fs::File::create(&temp_path)?;
        let config = serde_json::to_string_pretty(&self.config).unwrap_or("".to_string());
        file.write_all(config.as_bytes())?;
        file.sync_all()?;
        drop(file);

        if backup_path.exists() {
            std::fs::remove_file(&backup_path)?;
        }
        if path.exists() {
            std::fs::rename(&path, &backup_path)?;
        }
        if let Err(error) = std::fs::rename(&temp_path, &path) {
            if backup_path.exists() {
                std::fs::rename(&backup_path, &path).map_err(|restore_error| {
                    anyhow::anyhow!(
                        "failed to replace config: {error}; failed to restore backup: {restore_error}"
                    )
                })?;
            }
            return Err(error.into());
        }
        if backup_path.exists() {
            let _ = std::fs::remove_file(backup_path);
        }
        Ok(())
    }
}

impl ConfigData {
    pub fn plugin_settings(&self, plugin_id: &str) -> Option<Value> {
        self.plugins.get(plugin_id).cloned()
    }

    pub fn set_plugin_settings(&mut self, plugin_id: &str, values: Value) {
        self.plugins.insert(plugin_id.to_string(), values);
    }

    pub fn remove_plugin_settings(&mut self, plugin_id: &str) -> bool {
        self.plugins.remove(plugin_id).is_some()
    }
}
pub fn save_setting_data(setting_info: Value) -> Result<(String, String)> {
    let mut config = Config::new();
    if let Some(value) = setting_info.get("hotkeyAwaken").and_then(Value::as_str) {
        config.update_local_config(ConfigUpdate::HotkeyAwaken(value.to_string()));
    }
    if let Some(value) = setting_info.get("hotkeyClipboard").and_then(Value::as_str) {
        config.update_local_config(ConfigUpdate::HotkeyClipboard(value.to_string()));
    }
    if let Some(value) = setting_info.get("hotkeyFileJump").and_then(Value::as_str) {
        config.update_local_config(ConfigUpdate::HotkeyFileJump(value.to_string()));
    }
    // 与其它字段不同：这里按「key 是否存在」判断，而不是 as_str()。
    // 前端未加载成功时不传该 key，就不能覆盖已有配置；需要清空时必须显式传 null
    //（undefined 会被 JSON.stringify 整个丢弃，等同于「不传」）。
    if let Some(value) = setting_info.get("pythonInterpreter") {
        let configured = value
            .as_str()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string);
        config.update_local_config(ConfigUpdate::PythonInterpreter(configured));
    }
    if let Some(value) = setting_info
        .get("clipboardCountSwitch")
        .and_then(Value::as_bool)
    {
        config.update_local_config(ConfigUpdate::ClipboardRecordCountSwitch(value));
    }
    if let Some(value) = setting_info.get("clipboardCount").and_then(Value::as_i64) {
        config.update_local_config(ConfigUpdate::ClipboardRecordCount(Some(
            value.clamp(10, 200) as i32,
        )));
    }
    if let Some(value) = setting_info
        .get("clipboardTextSwitch")
        .and_then(Value::as_bool)
    {
        config.update_local_config(ConfigUpdate::ClipboardRecordTextSwitch(value));
    }
    if let Some(value) = setting_info.get("clipboardText").and_then(Value::as_i64) {
        config.update_local_config(ConfigUpdate::ClipboardRecordTextTime(Some(
            value.clamp(1, 30) as i32,
        )));
    }
    if let Some(value) = setting_info
        .get("clipboardImageSwitch")
        .and_then(Value::as_bool)
    {
        config.update_local_config(ConfigUpdate::ClipboardRecordImageSwitch(value));
    }
    if let Some(value) = setting_info.get("clipboardImage").and_then(Value::as_i64) {
        config.update_local_config(ConfigUpdate::ClipboardRecordImageTime(Some(
            value.clamp(1, 15) as i32,
        )));
    }
    if let Some(value) = setting_info
        .get("clipboardFileSwitch")
        .and_then(Value::as_bool)
    {
        config.update_local_config(ConfigUpdate::ClipboardRecordFileSwitch(value));
    }
    if let Some(value) = setting_info.get("clipboardFile").and_then(Value::as_i64) {
        config.update_local_config(ConfigUpdate::ClipboardRecordFileTime(Some(
            value.clamp(1, 10) as i32,
        )));
    }
    config.config.base.clipboard_settings_initialized = true;
    config.save_local_config()?;
    Ok((
        config.config.base.hotkey_awaken.clone(),
        config.config.base.hotkey_clipboard.clone(),
    ))
}

pub fn app_settings() -> Result<Value> {
    let config = Config::read_local_config()?;
    let base = config.base;
    Ok(serde_json::json!({
        "hotkeyAwaken": base.hotkey_awaken,
        "hotkeyClipboard": base.hotkey_clipboard,
        "hotkeyFileJump": base.hotkey_file_jump,
        "clipboardCountSwitch": base.clipboard_record_count_switch,
        "clipboardCount": base.clipboard_record_count,
        "clipboardTextSwitch": base.clipboard_record_text_switch,
        "clipboardText": base.clipboard_record_text_time,
        "clipboardImageSwitch": base.clipboard_record_image_switch,
        "clipboardImage": base.clipboard_record_image_time,
        "clipboardFileSwitch": base.clipboard_record_file_switch,
        "clipboardFile": base.clipboard_record_file_time,
        "pythonInterpreter": base.python_interpreter,
    }))
}

pub fn snippet_settings() -> Result<Value> {
    let config = Config::read_local_config()?;
    Ok(serde_json::json!({
        "enabled": config.base.snippets_enabled,
        "trigger": config.base.snippet_trigger,
        "snippets": config.base.text_snippets,
    }))
}

pub fn save_snippet_settings_data(setting_info: Value) -> Result<(bool, String, Vec<TextSnippet>)> {
    let enabled = setting_info
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let trigger = setting_info
        .get("trigger")
        .and_then(Value::as_str)
        .unwrap_or(";")
        .trim()
        .to_string();
    if trigger.chars().count() != 1 || trigger.chars().any(char::is_whitespace) {
        return Err(anyhow::anyhow!("触发符必须是一个非空白字符"));
    }

    let snippets: Vec<TextSnippet> = serde_json::from_value(
        setting_info
            .get("snippets")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    )?;
    for snippet in &snippets {
        if snippet.keyword.is_empty()
            || snippet.keyword.len() > 32
            || !snippet
                .keyword
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err(anyhow::anyhow!(
                "关键词只能包含字母、数字、下划线或短横线，长度为 1 到 32"
            ));
        }
        if snippet.text.is_empty() {
            return Err(anyhow::anyhow!("片段内容不能为空"));
        }
    }
    for (index, snippet) in snippets.iter().enumerate() {
        if snippets[..index]
            .iter()
            .any(|other| other.keyword == snippet.keyword)
        {
            return Err(anyhow::anyhow!("关键词不能重复：{}", snippet.keyword));
        }
        if snippets.iter().enumerate().any(|(other_index, other)| {
            other_index != index
                && other.keyword.starts_with(&snippet.keyword)
                && other.keyword != snippet.keyword
        }) {
            return Err(anyhow::anyhow!(
                "关键词不能互为前缀：{}，否则无法判断何时展开",
                snippet.keyword
            ));
        }
    }

    let mut config = Config::new();
    config.config.base.snippets_enabled = enabled;
    config.config.base.snippet_trigger = trigger.clone();
    config.config.base.text_snippets = snippets.clone();
    config.save_local_config()?;
    Ok((enabled, trigger, snippets))
}

pub fn ensure_file_search_paths_initialized(default_paths: Vec<String>) -> Result<Vec<String>> {
    let mut config = Config::new();
    if let Some(paths) = config.config.base.local_file_search_paths.clone() {
        return Ok(paths);
    }

    config.update_local_config(ConfigUpdate::LocalFileSearchPaths(Some(
        default_paths.clone(),
    )));
    config.save_local_config()?;
    Ok(default_paths)
}

pub fn index_initialization_flags_present() -> Result<(bool, bool)> {
    let path = config_path()?;
    if !path.exists() {
        return Ok((false, false));
    }
    let value: Value = serde_json::from_reader(std::fs::File::open(path)?)?;
    let base = value.get("base").and_then(Value::as_object);
    Ok((
        base.is_some_and(|base| base.contains_key("app_index_initialized")),
        base.is_some_and(|base| base.contains_key("file_index_initialized")),
    ))
}

pub fn save_index_initialization_flags(
    app_initialized: Option<bool>,
    file_initialized: Option<bool>,
) -> Result<()> {
    let mut config = Config::new();
    if let Some(value) = app_initialized {
        config.config.base.app_index_initialized = value;
    }
    if let Some(value) = file_initialized {
        config.config.base.file_index_initialized = value;
    }
    config.save_local_config()
}

pub fn save_index_settings_data(setting_info: Value) -> Result<()> {
    let mut config = Config::new();
    if setting_info.get("localFileSearchPaths").is_some() {
        let paths = setting_info
            .get("localFileSearchPaths")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            });
        config.update_local_config(ConfigUpdate::LocalFileSearchPaths(paths));
    }
    if let Some(value) = setting_info
        .get("localAppSearchPaths")
        .and_then(Value::as_array)
    {
        config.update_local_config(ConfigUpdate::LocalAppSearchPaths(
            value
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ));
    }
    if let Some(value) = setting_info
        .get("localAppSearchExcludePaths")
        .and_then(Value::as_array)
    {
        config.update_local_config(ConfigUpdate::LocalAppSearchExcludePaths(
            value
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ));
    }
    if let Some(value) = setting_info
        .get("localFileSearchExcludePaths")
        .and_then(Value::as_array)
    {
        config.update_local_config(ConfigUpdate::LocalFileSearchExcludePaths(
            value
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ));
    }
    if let Some(value) = setting_info
        .get("localFileSearchExcludeTypes")
        .and_then(Value::as_array)
    {
        config.update_local_config(ConfigUpdate::LocalFileSearchExcludeTypes(
            value
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ));
    }
    config.save_local_config()
}

pub fn index_settings() -> Result<Value> {
    let config = Config::read_local_config()?;
    Ok(serde_json::json!({
        "localAppSearchPaths": config.base.local_app_search_paths,
        "localAppSearchExcludePaths": config.base.local_app_search_exclude_paths,
        "localFileSearchPaths": config.base.local_file_search_paths,
        "localFileSearchExcludePaths": config.base.local_file_search_exclude_paths,
        "localFileSearchExcludeTypes": config.base.local_file_search_exclude_types,
    }))
}

pub fn hotkey_settings() -> Result<(String, String, String)> {
    let config = Config::read_local_config()?;
    Ok((
        config.base.hotkey_awaken,
        config.base.hotkey_clipboard,
        config.base.hotkey_file_jump,
    ))
}

/// 读取某个插件的配置值；没有记录时返回空对象，便于前端直接展开。
pub fn plugin_settings(plugin_id: &str) -> Result<Value> {
    let config = Config::read_local_config()?;
    Ok(config
        .plugin_settings(plugin_id)
        .unwrap_or_else(|| serde_json::json!({})))
}

/// 读取全部插件的配置值，供完整性摘要计算使用（不经过前端内存）。
pub fn plugin_settings_map() -> Result<HashMap<String, Value>> {
    let config = Config::read_local_config()?;
    Ok(config.plugins.clone())
}

/// 覆盖写入某个插件的配置值。键的合法性由命令层按 manifest 声明过滤。
pub fn save_plugin_settings_data(plugin_id: &str, values: Value) -> Result<()> {
    let mut config = Config::new();
    config.config.set_plugin_settings(plugin_id, values);
    config.save_local_config()
}

pub fn clear_plugin_settings_data(plugin_id: &str) -> Result<()> {
    let mut config = Config::new();
    config.config.remove_plugin_settings(plugin_id);
    config.save_local_config()
}

#[test]
fn te() {
    fn should_skip_dir(entry: &str, skip_dirs: &[String]) -> bool {
        let mut is_skip = false;
        if !is_skip {
            is_skip = skip_dirs.iter().any(|dir| {
                if dir.starts_with("*/") {
                    let skip_key = dir.split("/").last().clone().unwrap();
                    entry.contains(skip_key)
                } else {
                    false
                }
            });
        }
        is_skip
    }
    println!(
        "{:?}",
        should_skip_dir(
            "/Users/starsxu/Develop/Project/jade-smoke/node_modules",
            &vec!["*/node_modules".to_string()]
        )
    );
}

#[cfg(test)]
mod plugin_config_tests {
    use super::*;

    #[test]
    fn config_data_without_plugins_key_keeps_base_settings() {
        let mut value = serde_json::to_value(ConfigData::default()).expect("配置应可序列化");
        let removed = value
            .as_object_mut()
            .expect("配置根节点应为对象")
            .remove("plugins");
        assert!(
            removed.is_some(),
            "默认配置应包含 plugins 键，否则本测试不再验证缺键场景"
        );
        let restored: ConfigData =
            serde_json::from_value(value).expect("缺少 plugins 键不应导致整个配置回退默认值");
        assert!(restored.plugins.is_empty());
        assert_eq!(restored.base.app_name, "lark");
    }

    #[test]
    fn config_without_index_flags_defaults_to_not_initialized() {
        let mut value = serde_json::to_value(ConfigData::default()).expect("配置应可序列化");
        let base = value
            .get_mut("base")
            .and_then(Value::as_object_mut)
            .expect("base 应为对象");
        base.remove("app_index_initialized");
        base.remove("file_index_initialized");

        let restored: ConfigData = serde_json::from_value(value).expect("旧配置应可读取");
        assert!(!restored.base.app_index_initialized);
        assert!(!restored.base.file_index_initialized);
    }
}
