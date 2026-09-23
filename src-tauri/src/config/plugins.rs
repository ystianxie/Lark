use crate::utils::dirs::app_plugins_dir;
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

static CREATE_PLUGIN_LOCK: Mutex<()> = Mutex::new(());

/// 除图标外的固定文件。图标允许 SVG 或受支持的位图，因此单独列出。
const PLUGIN_FILES: [&str; 5] = [
    "manifest.json",
    "dist/main.js",
    "README.md",
    "scaffold.json",
    "python/main.py",
];

/// 允许写入的图标文件。
const ICON_FILES: [&str; 5] = [
    "assets/icon.svg",
    "assets/icon.png",
    "assets/icon.jpg",
    "assets/icon.webp",
    "assets/icon.ico",
];

/// 位图图标以 base64 文本传输，落盘前需要解码；SVG 直接按文本写入。
const BASE64_ICON_FILES: [&str; 4] = [
    "assets/icon.png",
    "assets/icon.jpg",
    "assets/icon.webp",
    "assets/icon.ico",
];

/// 图标体积上限，与前端校验保持一致。
const ICON_MAX_BYTES: usize = 256 * 1024;

fn is_plugin_file(name: &str) -> bool {
    PLUGIN_FILES.contains(&name) || ICON_FILES.contains(&name)
}

/// 把上传的图标内容转成落盘字节：位图先解码 base64，再校验文件头与体积，
/// 避免把伪装成 .png 的其它内容写进插件资源目录。
fn icon_file_bytes(name: &str, content: &str) -> Result<Vec<u8>, String> {
    if !BASE64_ICON_FILES.contains(&name) {
        return Ok(content.as_bytes().to_vec());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content.trim())
        .map_err(|error| format!("{name} 不是有效的 base64：{error}"))?;
    let matched = match name {
        "assets/icon.png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "assets/icon.jpg" => bytes.starts_with(b"\xff\xd8\xff"),
        "assets/icon.webp" => {
            bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP"
        }
        "assets/icon.ico" => bytes.starts_with(b"\x00\x00\x01\x00"),
        _ => true,
    };
    if !matched {
        return Err(format!("{name} 的内容与扩展名不符"));
    }
    if bytes.len() > ICON_MAX_BYTES {
        return Err(format!("{name} 超过 {} KB", ICON_MAX_BYTES / 1024));
    }
    Ok(bytes)
}

pub fn valid_plugin_id(id: &str) -> bool {
    let reserved = ["con", "prn", "aux", "nul"];
    !id.is_empty()
        && id.len() <= 64
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
        && !reserved.contains(&id)
        && !(id.len() == 4
            && (id.starts_with("com") || id.starts_with("lpt"))
            && id.as_bytes()[3].is_ascii_digit())
}

/// 配置项 key 的规则刻意与 [`valid_plugin_id`] 分开：它只是 JSON 对象键，
/// 不会变成目录名，因此不需要路径安全那套约束，也就没有理由禁止下划线。
/// 限定为蛇形命名，让插件在 JS 侧可以点号访问（`config.api_key`）；
/// Python 侧读字典本来就用 `config["api_key"]` 或 `config.get("api_key")`。
pub fn valid_config_key(key: &str) -> bool {
    if key.is_empty() || key.len() > 32 {
        return false;
    }
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validate_plugin_files(plugin_id: &str, files: &BTreeMap<String, String>) -> Result<(), String> {
    if !valid_plugin_id(plugin_id) {
        return Err("插件 ID 无效或为系统保留名称".into());
    }
    if files.keys().any(|name| !is_plugin_file(name)) {
        return Err("插件包含不允许写入的文件路径".into());
    }
    if files.values().any(|text| text.len() > 2 * 1024 * 1024)
        || files.values().map(String::len).sum::<usize>() > 8 * 1024 * 1024
    {
        return Err("插件模板内容过大".into());
    }
    for required in &PLUGIN_FILES[..4] {
        if files
            .get(*required)
            .map_or(true, |content| content.trim().is_empty())
        {
            return Err(format!("缺少文件：{required}"));
        }
    }
    // 图标至少提供一种格式；位图在这里完成 base64 解码与文件头校验，写入阶段直接使用结果。
    if !ICON_FILES.iter().any(|name| {
        files
            .get(*name)
            .is_some_and(|content| !content.trim().is_empty())
    }) {
        return Err("缺少图标文件".into());
    }
    for name in ICON_FILES {
        if let Some(content) = files.get(name) {
            if !content.trim().is_empty() {
                icon_file_bytes(name, content)?;
            }
        }
    }
    let manifest: Value = serde_json::from_str(&files["manifest.json"])
        .map_err(|error| format!("Manifest 格式错误：{error}"))?;
    if manifest["id"].as_str() != Some(plugin_id)
        || manifest["apiVersion"] != 1
        || manifest["entry"]["type"] != "js"
        || manifest["entry"]["path"] != "dist/main.js"
        || manifest["version"] != "1.0.0"
        || ["name", "description"].iter().any(|key| {
            manifest[key]
                .as_str()
                .map_or(true, |text| text.trim().is_empty())
        })
    {
        return Err("Manifest 与创建参数不一致或缺少必要字段".into());
    }
    let workflows = manifest["workflows"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or("缺少功能入口")?;
    let permissions = manifest["permissions"].as_array().ok_or("缺少权限声明")?;
    if permissions.iter().any(|permission| {
        !matches!(
            permission.as_str(),
            Some(
                "url.open"
                    | "file.open"
                    | "clipboard.read"
                    | "clipboard.write"
                    | "python.execute"
                    | "notification.send"
            )
        )
    }) {
        return Err("不支持的权限声明".into());
    }
    let mut ids = HashSet::new();
    for workflow in workflows {
        let id = workflow["id"].as_str().ok_or("缺少功能 ID")?;
        if !valid_plugin_id(id) || !ids.insert(id) {
            return Err("功能 ID 无效或重复".into());
        }
        if workflow["title"]
            .as_str()
            .map_or(true, |text| text.trim().is_empty())
        {
            return Err("缺少功能名称".into());
        }
        let keywords = workflow["keywords"]
            .as_array()
            .filter(|items| !items.is_empty())
            .ok_or("缺少功能关键词")?;
        if keywords
            .iter()
            .any(|keyword| keyword.as_str().map_or(true, |text| text.trim().is_empty()))
        {
            return Err("功能关键词无效".into());
        }
        match workflow["type"].as_str() {
            Some("url") => {
                let url = workflow["data"].as_str().ok_or("缺少目标网址")?;
                let parsed = tauri::Url::parse(url).map_err(|_| "目标网址无效")?;
                if ![
                    "http",
                    "https",
                    "mailto",
                    "chrome-extension",
                    "moz-extension",
                ]
                .contains(&parsed.scheme())
                {
                    return Err("不支持的网址协议".into());
                }
            }
            Some("action" | "python") => {
                if workflow["handler"].as_str() != Some(id) {
                    return Err("功能 handler 必须与 ID 一致".into());
                }
                if workflow["type"] == "python"
                    && (!permissions.iter().any(|value| value == "python.execute")
                        || files
                            .get("python/main.py")
                            .map_or(true, |text| text.trim().is_empty()))
                {
                    return Err("Python 功能缺少脚本或执行权限".into());
                }
            }
            _ => return Err("创建向导只支持 action、url、python".into()),
        }
    }
    validate_plugin_config(&manifest, &ids)?;
    let scaffold: Value = serde_json::from_str(&files["scaffold.json"])
        .map_err(|error| format!("生成配置格式错误：{error}"))?;
    validate_scaffold(plugin_id, &scaffold)?;
    Ok(())
}

/// 校验可选的 `config` 声明。缺失该字段时插件行为与旧版完全一致。
///
/// 注意：本函数只在向导创建/更新路径上执行，`load_plugins` 直接透传 manifest，
/// 因此手动安装的插件可能带任意 `config`，前端渲染必须自带防御。
fn validate_plugin_config(manifest: &Value, workflow_ids: &HashSet<&str>) -> Result<(), String> {
    let Some(config) = manifest.get("config") else {
        return Ok(());
    };
    if config.is_null() {
        return Ok(());
    }
    let config = config.as_object().ok_or("config 必须是对象")?;
    if config.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return Err("config.schemaVersion 必须为 1".into());
    }
    let ui = config.get("ui").and_then(Value::as_str).unwrap_or("form");
    if !matches!(ui, "form" | "custom") {
        return Err("config.ui 只支持 form 或 custom".into());
    }
    if ui == "custom" {
        let settings_page = config
            .get("settingsPage")
            .and_then(Value::as_str)
            .ok_or("config.ui 为 custom 时必须提供 settingsPage")?;
        if !workflow_ids.iter().any(|id| *id == settings_page) {
            return Err("config.settingsPage 必须指向已存在的功能入口".into());
        }
    }
    let fields = config
        .get("fields")
        .and_then(Value::as_array)
        .ok_or("config.fields 必须是数组")?;
    if fields.is_empty() {
        return Err("config.fields 不能为空".into());
    }
    if fields.len() > 24 {
        return Err("config.fields 最多 24 项".into());
    }
    let mut keys = HashSet::new();
    for field in fields {
        let key = field
            .get("key")
            .and_then(Value::as_str)
            .ok_or("配置项缺少 key")?;
        if !valid_config_key(key) {
            return Err(
                "配置项 key 只允许小写字母开头的蛇形命名（小写字母、数字、下划线），最多 32 字符"
                    .into(),
            );
        }
        if !keys.insert(key.to_string()) {
            return Err("配置项 key 不能重复".into());
        }
        if field
            .get("label")
            .and_then(Value::as_str)
            .map_or(true, |label| label.trim().is_empty())
        {
            return Err("配置项缺少 label".into());
        }
        let field_type = field
            .get("type")
            .and_then(Value::as_str)
            .ok_or("配置项缺少 type")?;
        if ![
            "text", "password", "number", "boolean", "select", "textarea",
        ]
        .contains(&field_type)
        {
            return Err("不支持的配置项类型".into());
        }
        if field_type == "select"
            && field
                .get("options")
                .and_then(Value::as_array)
                .map_or(true, |options| options.is_empty())
        {
            return Err("select 配置项必须提供非空 options".into());
        }
        if let Some(default) = field.get("default") {
            let matches_type = match field_type {
                "text" | "password" | "textarea" => default.is_string(),
                "number" => default.is_number(),
                "boolean" => default.is_boolean(),
                "select" => default.is_string() || default.is_number() || default.is_boolean(),
                _ => false,
            };
            if !matches_type {
                return Err("配置项 default 与 type 不匹配".into());
            }
        }
    }
    Ok(())
}

fn validate_scaffold(plugin_id: &str, scaffold: &Value) -> Result<(), String> {
    if scaffold["schemaVersion"] != 1
        || scaffold["id"].as_str() != Some(plugin_id)
        || scaffold["name"]
            .as_str()
            .map_or(true, |text| text.trim().is_empty())
        || scaffold["description"]
            .as_str()
            .map_or(true, |text| text.trim().is_empty())
        || scaffold["workflows"]
            .as_array()
            .map_or(true, |items| items.is_empty())
    {
        return Err("scaffold.json 不是有效的向导配置".into());
    }
    // scaffold.json 保存的是同一份向导配置，配置声明必须与 manifest 一致。
    let workflow_ids: HashSet<&str> = scaffold["workflows"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["id"].as_str())
                .collect()
        })
        .unwrap_or_default();
    validate_plugin_config(scaffold, &workflow_ids)?;
    Ok(())
}

fn write_plugin_files(
    root: &Path,
    plugin_id: &str,
    files: &BTreeMap<String, String>,
) -> Result<PathBuf, String> {
    validate_plugin_files(plugin_id, files)?;
    let destination = root.join(plugin_id);
    fs::create_dir(&destination)
        .map_err(|error| format!("无法创建插件目录（不会覆盖已有目录）：{error}"))?;
    let write = || -> std::io::Result<()> {
        fs::create_dir(destination.join("dist"))?;
        fs::create_dir(destination.join("assets"))?;
        if files.contains_key("python/main.py") {
            fs::create_dir(destination.join("python"))?;
        }
        for (name, content) in files
            .iter()
            .filter(|(name, _)| name.as_str() != "manifest.json")
        {
            // 图标可能是 base64 编码的位图，统一经由 icon_file_bytes 取落盘字节。
            let bytes = icon_file_bytes(name, content)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination.join(name))?;
            file.write_all(&bytes)?;
        }
        let temporary_manifest = destination.join("manifest.json.tmp");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_manifest)?;
        file.write_all(files["manifest.json"].as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary_manifest, destination.join("manifest.json"))?;
        Ok(())
    };
    if let Err(error) = write() {
        return match fs::remove_dir_all(&destination) {
            Ok(()) => Err(format!("写入插件失败，已清理本次创建内容：{error}")),
            Err(cleanup) => Err(format!(
                "写入插件失败：{error}；清理目录 {} 失败：{cleanup}",
                destination.display()
            )),
        };
    }
    Ok(destination)
}

#[tauri::command]
pub fn create_plugin(
    app: AppHandle,
    plugin_id: String,
    files: BTreeMap<String, String>,
) -> Result<String, String> {
    let _guard = CREATE_PLUGIN_LOCK
        .lock()
        .map_err(|_| "插件创建服务不可用")?;
    validate_plugin_files(&plugin_id, &files)?;
    if load_plugins(app.clone())
        .iter()
        .any(|plugin| plugin.id.eq_ignore_ascii_case(&plugin_id))
    {
        return Err("插件 ID 已存在，请更换 ID".into());
    }
    let root = app_plugins_dir().map_err(|error| error.to_string())?;
    app.asset_protocol_scope()
        .allow_directory(&root, true)
        .map_err(|error| format!("无法授权插件资源目录：{error}"))?;
    let destination = write_plugin_files(&root, &plugin_id, &files)?;
    Ok(normalize_asset_path(&destination.to_string_lossy()))
}

#[tauri::command]
pub fn load_plugin_editor(plugin_id: String) -> Result<Value, String> {
    if !valid_plugin_id(&plugin_id) {
        return Err("插件 ID 无效".into());
    }
    let root = app_plugins_dir()
        .map_err(|e| e.to_string())?
        .join(&plugin_id);
    let scaffold = fs::read_to_string(root.join("scaffold.json"))
        .map_err(|_| "该插件不是向导创建的插件".to_string())?;
    let mut value: Value =
        serde_json::from_str(&scaffold).map_err(|e| format!("scaffold.json 无效：{e}"))?;
    validate_scaffold(&plugin_id, &value)?;
    if value["icon"].as_str().map_or(true, |s| s.is_empty()) {
        value["icon"] = Value::String(read_icon_for_editor(&root));
    }
    Ok(value)
}

/// 回填图标供编辑器预览：优先 SVG 文本，否则把位图读成 data URL。
fn read_icon_for_editor(root: &Path) -> String {
    if let Ok(text) = fs::read_to_string(root.join("assets/icon.svg")) {
        if !text.trim().is_empty() {
            return text;
        }
    }
    for (name, mime) in [
        ("assets/icon.png", "image/png"),
        ("assets/icon.jpg", "image/jpeg"),
        ("assets/icon.webp", "image/webp"),
        ("assets/icon.ico", "image/x-icon"),
    ] {
        if let Ok(bytes) = fs::read(root.join(name)) {
            if !bytes.is_empty() {
                return format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                );
            }
        }
    }
    String::new()
}

#[tauri::command]
pub fn update_plugin(plugin_id: String, files: BTreeMap<String, String>) -> Result<String, String> {
    let _guard = CREATE_PLUGIN_LOCK.lock().map_err(|_| "插件服务不可用")?;
    if !valid_plugin_id(&plugin_id) {
        return Err("插件 ID 无效".into());
    }
    let root = app_plugins_dir()
        .map_err(|e| e.to_string())?
        .join(&plugin_id);
    if !root.is_dir() || !root.join("scaffold.json").is_file() {
        return Err("仅支持编辑向导创建的插件".into());
    }
    validate_plugin_files(&plugin_id, &files)?;
    let previous: BTreeMap<&str, Option<Vec<u8>>> = PLUGIN_FILES
        .iter()
        .chain(ICON_FILES.iter())
        .map(|name| (*name, fs::read(root.join(name)).ok()))
        .collect();
    let write = || -> Result<(), String> {
        if files.contains_key("python/main.py") {
            fs::create_dir_all(root.join("python"))
                .map_err(|e| format!("创建 Python 目录失败：{e}"))?;
        }
        for (name, content) in files
            .iter()
            .filter(|(name, _)| name.as_str() != "manifest.json")
        {
            // 位图图标以 base64 文本传输，落盘前解码成二进制。
            let bytes = icon_file_bytes(name, content)
                .map_err(|error| format!("写入 {name} 失败：{error}"))?;
            fs::write(root.join(name), bytes).map_err(|e| format!("写入 {name} 失败：{e}"))?;
        }
        if !files.contains_key("python/main.py") {
            match fs::remove_file(root.join("python/main.py")) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("删除旧 Python 脚本失败：{error}")),
            }
        }
        // 切换图标格式（例如 png 换回 svg）后，本次未提交的旧图标要删掉，避免残留两个文件。
        for icon in ICON_FILES {
            if files
                .get(icon)
                .is_some_and(|content| !content.trim().is_empty())
            {
                continue;
            }
            match fs::remove_file(root.join(icon)) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("删除旧图标失败：{error}")),
            }
        }
        fs::write(root.join("manifest.json"), &files["manifest.json"])
            .map_err(|e| format!("写入 manifest.json 失败：{e}"))?;
        Ok(())
    };
    if let Err(error) = write() {
        let mut rollback_errors = Vec::new();
        for (name, content) in previous {
            let path = root.join(name);
            let result = match content {
                Some(bytes) => fs::write(&path, bytes),
                None => match fs::remove_file(&path) {
                    Ok(()) => Ok(()),
                    Err(remove_error) if remove_error.kind() == std::io::ErrorKind::NotFound => {
                        Ok(())
                    }
                    Err(remove_error) => Err(remove_error),
                },
            };
            if let Err(rollback_error) = result {
                rollback_errors.push(format!("{name}: {rollback_error}"));
            }
        }
        return if rollback_errors.is_empty() {
            Err(format!("保存插件失败，已恢复原文件：{error}"))
        } else {
            Err(format!(
                "保存插件失败：{error}；恢复部分文件失败：{}",
                rollback_errors.join("；")
            ))
        };
    }
    Ok(normalize_asset_path(&root.to_string_lossy()))
}

#[tauri::command]
pub fn load_plugins(app: AppHandle) -> Vec<PluginRecord> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let user_plugins_dir = app_plugins_dir().ok();
    if let Some(dir) = &user_plugins_dir {
        dirs.push(dir.clone());
    }
    // Development convenience: load plugins checked into the repository.
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../plugins"));
    }
    if let Ok(dir) = app.path().resource_dir() {
        dirs.push(dir.join("plugins"));
    }
    let mut result = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }
            let canonical_path = fs::canonicalize(&path).unwrap_or(path.clone());
            let root = normalize_asset_path(&canonical_path.to_string_lossy());
            let fallback_id = entry.file_name().to_string_lossy().to_string();
            match fs::read_to_string(&manifest_path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
            {
                Ok(manifest) => {
                    let id = manifest
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(&fallback_id)
                        .to_string();
                    if !result.iter().any(|item: &PluginRecord| item.id == id) {
                        let editable = user_plugins_dir.as_ref().is_some_and(|root| root == &dir)
                            && fs::read_to_string(path.join("scaffold.json"))
                                .ok()
                                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                                .is_some_and(|scaffold| validate_scaffold(&id, &scaffold).is_ok());
                        result.push(PluginRecord {
                            id,
                            root,
                            manifest,
                            error: None,
                            editable,
                        });
                    }
                }
                Err(error) => result.push(PluginRecord {
                    id: fallback_id,
                    root,
                    manifest: Value::Null,
                    error: Some(error),
                    editable: false,
                }),
            }
        }
    }
    result
}

fn normalize_asset_path(path: &str) -> String {
    let without_extended_prefix = path.strip_prefix("\\\\?\\").unwrap_or(path);
    without_extended_prefix.replace('\\', "/")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRecord {
    pub id: String,
    pub root: String,
    pub manifest: Value,
    pub error: Option<String>,
    pub editable: bool,
}

#[cfg(test)]
mod creation_tests {
    use super::*;
    // 匿名导入：super::* 可能已经带进 Engine，用 as _ 避免重名。
    use base64::Engine as _;

    fn files() -> BTreeMap<String, String> {
        let manifest = serde_json::json!({
            "id": "test-plugin", "name": "测试", "description": "测试插件", "version": "1.0.0", "apiVersion": 1,
            "entry": {"type": "js", "path": "dist/main.js"}, "permissions": [],
            "workflows": [{"id": "convert", "title": "转换", "type": "action", "handler": "convert", "keywords": ["test"]}]
        });
        // validate_plugin_files 末尾会校验 scaffold.json，因此 fixture 必须是合法的向导配置，
        // 否则所有「合法输入应通过」的断言都会因为这一步提前失败。
        let scaffold = serde_json::json!({
            "schemaVersion": 1, "id": "test-plugin", "name": "测试", "description": "测试插件",
            "workflows": [
                {"id": "convert", "title": "转换", "type": "action", "handler": "convert", "keywords": ["test"]}
            ]
        });
        BTreeMap::from([
            ("manifest.json".into(), manifest.to_string()),
            (
                "dist/main.js".into(),
                "export async function activate() { return {}; }".into(),
            ),
            ("assets/icon.svg".into(), "<svg/>".into()),
            ("README.md".into(), "test".into()),
            ("scaffold.json".into(), scaffold.to_string()),
        ])
    }

    /// 用最小合法 manifest 承载给定的 config 声明并运行校验。
    fn validate_config(config: Value) -> Result<(), String> {
        let mut manifest = serde_json::json!({
            "id": "test-plugin", "name": "测试", "description": "测试插件", "version": "1.0.0", "apiVersion": 1,
            "entry": {"type": "js", "path": "dist/main.js"}, "permissions": [],
            "workflows": [
                {"id": "convert", "title": "转换", "type": "action", "handler": "convert", "keywords": ["test"]},
                {"id": "settings", "title": "设置", "type": "action", "handler": "settings", "keywords": ["test-settings"]}
            ]
        });
        if !config.is_null() {
            manifest["config"] = config;
        }
        let workflow_ids: HashSet<&str> = manifest["workflows"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["id"].as_str())
            .collect();
        validate_plugin_config(&manifest, &workflow_ids)
    }

    #[test]
    fn plugin_config_is_optional() {
        assert!(validate_config(Value::Null).is_ok());
    }

    #[test]
    fn config_key_rule_accepts_snake_case_only() {
        for key in ["a", "api_key", "base_url", "base_url_2", "a1_b2"] {
            assert!(valid_config_key(key), "{key} 应合法");
        }
        for key in [
            "", "1abc", "_abc", "apiKey", "base-url", "api key", "API_KEY",
        ] {
            assert!(!valid_config_key(key), "{key} 应被拒绝");
        }
        assert!(valid_config_key(&"a".repeat(32)));
        assert!(!valid_config_key(&"a".repeat(33)));
    }

    #[test]
    fn plugin_config_accepts_valid_declarations() {
        assert!(validate_config(serde_json::json!({
            "schemaVersion": 1,
            "fields": [
                {"key": "api_key", "label": "API Key", "type": "password", "required": true},
                {"key": "base_url", "label": "接口地址", "type": "text", "default": "https://example.com"},
                {"key": "model", "label": "模型", "type": "select", "options": ["a", "b"], "default": "a"},
                {"key": "timeout_seconds", "label": "超时", "type": "number", "default": 30, "min": 1, "max": 120},
                {"key": "enabled", "label": "启用", "type": "boolean", "default": true},
                {"key": "prompt", "label": "提示词", "type": "textarea"}
            ]
        }))
        .is_ok());
        // ui=custom 时必须指向存在的功能入口。
        assert!(validate_config(serde_json::json!({
            "schemaVersion": 1,
            "ui": "custom",
            "settingsPage": "settings",
            "fields": [{"key": "api_key", "label": "API Key", "type": "password"}]
        }))
        .is_ok());
    }

    #[test]
    fn plugin_config_rejects_invalid_declarations() {
        let cases = [
            (
                serde_json::json!({"fields": [{"key": "api_key", "label": "API Key", "type": "text"}]}),
                "schemaVersion",
            ),
            (
                serde_json::json!({"schemaVersion": 2, "fields": [{"key": "api_key", "label": "API Key", "type": "text"}]}),
                "schemaVersion",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": []}),
                "fields",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "ui": "wizard", "fields": [{"key": "api_key", "label": "API Key", "type": "text"}]}),
                "ui",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "ui": "custom", "fields": [{"key": "api_key", "label": "API Key", "type": "text"}]}),
                "settingsPage",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "ui": "custom", "settingsPage": "missing", "fields": [{"key": "api_key", "label": "API Key", "type": "text"}]}),
                "settingsPage",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "Bad Key", "label": "A", "type": "text"}]}),
                "key",
            ),
            // 配置项 key 限定蛇形，camelCase 与短横线都不接受。
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "apiKey", "label": "A", "type": "text"}]}),
                "key",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "base-url", "label": "A", "type": "text"}]}),
                "key",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [
                    {"key": "api_key", "label": "A", "type": "text"},
                    {"key": "api_key", "label": "B", "type": "text"}
                ]}),
                "重复",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "a", "label": "  ", "type": "text"}]}),
                "label",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "a", "label": "A", "type": "color"}]}),
                "类型",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "a", "label": "A", "type": "select"}]}),
                "options",
            ),
            (
                serde_json::json!({"schemaVersion": 1, "fields": [{"key": "a", "label": "A", "type": "number", "default": "30"}]}),
                "default",
            ),
        ];
        for (config, expected) in cases {
            let error = validate_config(config).unwrap_err();
            assert!(error.contains(expected), "{error} 应包含 {expected}");
        }
    }

    #[test]
    fn plugin_config_rejects_too_many_fields() {
        let fields: Vec<Value> = (0..25)
            .map(|index| {
                serde_json::json!({"key": format!("field_{index}"), "label": "F", "type": "text"})
            })
            .collect();
        assert!(
            validate_config(serde_json::json!({"schemaVersion": 1, "fields": fields})).is_err()
        );
    }

    fn base64_bytes(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn icon_bytes_decode_bitmap_and_validate_header_and_size() {
        let png = [0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3];
        assert_eq!(
            icon_file_bytes("assets/icon.png", &base64_bytes(&png)).unwrap(),
            png.to_vec()
        );
        // 位图文件头不匹配时要拒绝，防止把别的内容伪装成 .png 写进插件目录。
        assert!(icon_file_bytes("assets/icon.png", &base64_bytes(b"not a png")).is_err());
        assert!(icon_file_bytes("assets/icon.jpg", &base64_bytes(&png)).is_err());
        assert!(icon_file_bytes("assets/icon.png", "!!not-base64!!").is_err());
        // WebP 需要同时匹配 RIFF 与 WEBP 标记。
        assert!(icon_file_bytes(
            "assets/icon.webp",
            &base64_bytes(b"RIFF\x00\x00\x00\x00WEBP")
        )
        .is_ok());
        assert!(icon_file_bytes(
            "assets/icon.webp",
            &base64_bytes(b"RIFF\x00\x00\x00\x00XXXX")
        )
        .is_err());
        // SVG 是文本，不走 base64。
        assert_eq!(
            icon_file_bytes("assets/icon.svg", "<svg/>").unwrap(),
            b"<svg/>".to_vec()
        );
        // 超过上限（文件头正确，仅体积过大）。
        let mut oversized = vec![0u8; ICON_MAX_BYTES + 1];
        oversized[..8].copy_from_slice(&png[..8]);
        assert!(
            icon_file_bytes("assets/icon.png", &base64_bytes(&oversized))
                .unwrap_err()
                .contains("超过")
        );
    }

    #[test]
    fn plugin_creation_requires_an_icon_file() {
        let mut content = files();
        content.remove("assets/icon.svg");
        assert!(validate_plugin_files("test-plugin", &content)
            .unwrap_err()
            .contains("图标"));
        // 换成位图同样可用。
        content.insert(
            "assets/icon.png".into(),
            base64_bytes(&[0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1]),
        );
        assert!(validate_plugin_files("test-plugin", &content).is_ok());
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "lark-plugin-creation-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn plugin_creation_rejects_invalid_ids_and_paths() {
        for id in [
            "",
            "../escape",
            "a/b",
            "a\\b",
            "con",
            "com1",
            "nul",
            "UPPER",
            "a--b",
        ] {
            assert!(validate_plugin_files(id, &files()).is_err(), "{id}");
        }
        let mut invalid = files();
        invalid.insert("../outside.txt".into(), "bad".into());
        assert!(validate_plugin_files("test-plugin", &invalid).is_err());
    }

    #[test]
    fn plugin_creation_validates_workflows_and_permissions() {
        let mut content = files();
        let original: Value = serde_json::from_str(&content["manifest.json"]).unwrap();
        let mut invalid = original.clone();
        invalid["workflows"][0]["keywords"] = serde_json::json!([]);
        content.insert("manifest.json".into(), invalid.to_string());
        assert!(validate_plugin_files("test-plugin", &content).is_err());
        invalid = original.clone();
        invalid["workflows"]
            .as_array_mut()
            .unwrap()
            .push(original["workflows"][0].clone());
        content.insert("manifest.json".into(), invalid.to_string());
        assert!(validate_plugin_files("test-plugin", &content).is_err());
        invalid = original.clone();
        invalid["workflows"][0]["type"] = "url".into();
        invalid["workflows"][0]["data"] = "javascript:alert(1)".into();
        content.insert("manifest.json".into(), invalid.to_string());
        assert!(validate_plugin_files("test-plugin", &content).is_err());
        invalid = original;
        invalid["workflows"][0]["type"] = "python".into();
        content.insert("manifest.json".into(), invalid.to_string());
        assert!(validate_plugin_files("test-plugin", &content).is_err());
        invalid["permissions"] = serde_json::json!(["python.execute"]);
        content.insert("manifest.json".into(), invalid.to_string());
        content.insert("python/main.py".into(), "print('{}')".into());
        assert!(validate_plugin_files("test-plugin", &content).is_ok());
    }

    #[test]
    fn plugin_creation_never_overwrites_and_rejects_incomplete_files() {
        let root = TestDirectory::new();
        let content = files();
        let destination = write_plugin_files(&root.0, "test-plugin", &content).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("manifest.json")).unwrap(),
            content["manifest.json"]
        );
        assert!(!destination.join("manifest.json.tmp").exists());
        let mut replacement = content.clone();
        replacement.insert("dist/main.js".into(), "overwrite".into());
        assert!(write_plugin_files(&root.0, "test-plugin", &replacement).is_err());
        assert_eq!(
            fs::read_to_string(destination.join("dist/main.js")).unwrap(),
            content["dist/main.js"]
        );
        let empty_root = TestDirectory::new();
        replacement.remove("dist/main.js");
        assert!(write_plugin_files(&empty_root.0, "test-plugin", &replacement).is_err());
        assert_eq!(fs::read_dir(&empty_root.0).unwrap().count(), 0);
        assert!(
            write_plugin_files(&empty_root.0.join("missing"), "test-plugin", &content).is_err()
        );
    }

    #[test]
    fn plugin_creation_concurrent_requests_have_one_winner() {
        let root = TestDirectory::new();
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let path = root.0.clone();
                std::thread::spawn(move || {
                    write_plugin_files(&path, "test-plugin", &files()).is_ok()
                })
            })
            .collect();
        let successes = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().unwrap()))
            .sum::<usize>();
        assert_eq!(successes, 1);
        assert!(root.0.join("test-plugin/manifest.json").is_file());
    }
}
