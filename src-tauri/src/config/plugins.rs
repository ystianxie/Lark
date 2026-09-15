use crate::utils::dirs::app_plugins_dir;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn load_plugins(app: AppHandle) -> Vec<PluginRecord> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = app_plugins_dir() {
        dirs.push(dir);
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
        let Ok(entries) = fs::read_dir(dir) else {
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
                        result.push(PluginRecord {
                            id,
                            root,
                            manifest,
                            error: None,
                        });
                    }
                }
                Err(error) => result.push(PluginRecord {
                    id: fallback_id,
                    root,
                    manifest: Value::Null,
                    error: Some(error),
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
}
