use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use crate::utils::dirs::app_plugins_dir;

#[tauri::command]
pub fn load_plugins() -> HashMap<String, String> {
    let dir = app_plugins_dir().unwrap();
    let mut modules = HashMap::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let plugin_name = entry.file_name();
            if let Ok(plugin_entries) = fs::read_dir(path) {
                for plugin_entry in plugin_entries {
                    let plugin_entry = plugin_entry.unwrap();
                    if plugin_entry.file_name() == "info.json" {
                        let config = fs::read_to_string(plugin_entry.path());
                        if let Ok(config) = config {
                            modules.insert(plugin_name.clone().into_string().unwrap(), config);
                        }
                    }
                }
            }
        }
    }

    modules
}
