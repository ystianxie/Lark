use crate::utils::dirs::app_plugins_dir;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

static CREATE_PLUGIN_LOCK: Mutex<()> = Mutex::new(());
const PLUGIN_FILES: [&str; 6] = [
    "manifest.json",
    "dist/main.js",
    "assets/icon.svg",
    "README.md",
    "scaffold.json",
    "python/main.py",
];

fn valid_plugin_id(id: &str) -> bool {
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

fn validate_plugin_files(plugin_id: &str, files: &BTreeMap<String, String>) -> Result<(), String> {
    if !valid_plugin_id(plugin_id) {
        return Err("插件 ID 无效或为系统保留名称".into());
    }
    if files
        .keys()
        .any(|name| !PLUGIN_FILES.contains(&name.as_str()))
    {
        return Err("插件包含不允许写入的文件路径".into());
    }
    if files.values().any(|text| text.len() > 2 * 1024 * 1024)
        || files.values().map(String::len).sum::<usize>() > 8 * 1024 * 1024
    {
        return Err("插件模板内容过大".into());
    }
    for required in &PLUGIN_FILES[..5] {
        if files
            .get(*required)
            .map_or(true, |content| content.trim().is_empty())
        {
            return Err(format!("缺少文件：{required}"));
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
                "url.open" | "file.open" | "clipboard.read" | "clipboard.write" | "python.execute"
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
    let scaffold: Value = serde_json::from_str(&files["scaffold.json"])
        .map_err(|error| format!("生成配置格式错误：{error}"))?;
    validate_scaffold(plugin_id, &scaffold)?;
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
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination.join(name))?;
            file.write_all(content.as_bytes())?;
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
        value["icon"] =
            Value::String(fs::read_to_string(root.join("assets/icon.svg")).unwrap_or_default());
    }
    Ok(value)
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
            fs::write(root.join(name), content).map_err(|e| format!("写入 {name} 失败：{e}"))?;
        }
        if !files.contains_key("python/main.py") {
            match fs::remove_file(root.join("python/main.py")) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("删除旧 Python 脚本失败：{error}")),
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

    fn files() -> BTreeMap<String, String> {
        let manifest = serde_json::json!({
            "id": "test-plugin", "name": "测试", "description": "测试插件", "version": "1.0.0", "apiVersion": 1,
            "entry": {"type": "js", "path": "dist/main.js"}, "permissions": [],
            "workflows": [{"id": "convert", "title": "转换", "type": "action", "handler": "convert", "keywords": ["test"]}]
        });
        BTreeMap::from([
            ("manifest.json".into(), manifest.to_string()),
            (
                "dist/main.js".into(),
                "export async function activate() { return {}; }".into(),
            ),
            ("assets/icon.svg".into(), "<svg/>".into()),
            ("README.md".into(), "test".into()),
            ("scaffold.json".into(), "{}".into()),
        ])
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
