use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static APP_DIR: &str = "lark";
static CONFIG_FILE: &str = "config.json";
static LAYOUT_LOCK: Mutex<()> = Mutex::new(());

/// get the app home dir
pub fn app_home_dir() -> Result<PathBuf> {
    // Lark intentionally keeps its mutable files beside the installed binary.
    // This also makes the plugin directory visible and portable with the app.
    let exe = std::env::current_exe()?;
    let install_dir = exe
        .parent()
        .ok_or(anyhow::anyhow!("failed to get the application directory"))?;
    let _guard = LAYOUT_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("layout lock poisoned"))?;
    initialize_layout(install_dir)
}

fn initialize_layout(install_dir: &Path) -> Result<PathBuf> {
    let home = install_dir.join("resources");
    let marker = home.join("data/.layout-v2");
    std::fs::create_dir_all(home.join("data"))?;
    if !marker.exists() {
        let legacy = install_dir.join("config").join(APP_DIR);
        if legacy.is_dir() {
            copy_missing(&legacy.join(CONFIG_FILE), &home.join(CONFIG_FILE))?;
            copy_missing(&legacy.join("logs"), &home.join("data/logs"))?;
            let old_data = legacy.join("data");
            if old_data.is_dir() {
                for entry in std::fs::read_dir(&old_data)? {
                    let entry = entry?;
                    if entry.file_name() != "plugins" {
                        copy_missing(&entry.path(), &home.join("data").join(entry.file_name()))?;
                    }
                }
                // Never merge two plugins with the same id: a partial merge
                // could mix an old manifest with a newly bundled JS entry.
                let old_plugins = old_data.join("plugins");
                if old_plugins.is_dir() {
                    for entry in std::fs::read_dir(old_plugins)? {
                        let entry = entry?;
                        let destination = home.join("plugins").join(entry.file_name());
                        if !destination.exists() {
                            let staging =
                                home.join("data/plugin-migration").join(entry.file_name());
                            copy_missing(&entry.path(), &staging)?;
                            std::fs::create_dir_all(home.join("plugins"))?;
                            std::fs::rename(staging, destination)?;
                        } else {
                            eprintln!(
                                "迁移时保留现有插件 {}，旧版文件保留在 {}",
                                destination.display(),
                                entry.path().display()
                            );
                        }
                    }
                }
            }
        }
        // Keep the original directory as a backup. Mark only completed copies,
        // so an interrupted migration is retried without replacing new data.
        std::fs::write(marker, b"2\n")?;
    }
    Ok(home)
}

fn copy_missing(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(source)?;
    anyhow::ensure!(
        !metadata.file_type().is_symlink(),
        "cannot migrate symbolic link: {}",
        source.display()
    );
    if metadata.is_dir() {
        std::fs::create_dir_all(destination)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_missing(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if !destination.exists() {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = destination.with_extension("lark-migration-tmp");
        std::fs::copy(source, &temporary)?;
        std::fs::rename(temporary, destination)?;
    }
    Ok(())
}

/// Preserve localStorage (including disabled plugins) when moving WebView2's
/// profile out of LocalAppData. Publish only a completely copied profile.
pub fn webview_data_dir(legacy: Option<&Path>) -> Result<PathBuf> {
    let data = app_data_dir()?;
    initialize_webview_data(&data, legacy)
}

fn initialize_webview_data(data: &Path, legacy: Option<&Path>) -> Result<PathBuf> {
    let destination = data.join("webview");
    if !destination.exists() {
        let staging = data.join("webview-migration");
        if let Some(legacy) = legacy.filter(|root| root.join("EBWebView").is_dir()) {
            copy_missing(&legacy.join("EBWebView"), &staging.join("EBWebView"))?;
        }
        std::fs::create_dir_all(&staging)?;
        std::fs::rename(staging, &destination)?;
    }
    Ok(destination)
}

/// logs dir
#[allow(unused)]
pub fn app_logs_dir() -> Result<PathBuf> {
    let logs = app_data_dir()?.join("logs");
    if !logs.exists() {
        std::fs::create_dir(&logs)?;
    }
    Ok(logs)
}

pub fn config_path() -> Result<PathBuf> {
    let config = app_home_dir()?.join(CONFIG_FILE);
    Ok(config)
}

#[allow(unused)]
pub fn app_data_dir() -> Result<PathBuf> {
    let data = app_home_dir()?.join("data");
    if !data.exists() {
        std::fs::create_dir(&data)?;
    }
    Ok(data)
}

pub fn app_plugins_dir() -> Result<PathBuf> {
    let plugins = app_home_dir()?.join("plugins");
    if !plugins.exists() {
        std::fs::create_dir(&plugins)?;
    }
    Ok(plugins)
}
pub fn app_clipboard_img_dir() -> Result<PathBuf> {
    let clipboard_img_dir = app_data_dir()?.join("clipboardImg");
    if !clipboard_img_dir.exists() {
        std::fs::create_dir_all(&clipboard_img_dir)?;
    }
    Ok(clipboard_img_dir)
}
#[tauri::command]
pub fn get_app_dir() -> HashMap<String, String> {
    let log_dir = app_logs_dir().unwrap();
    let data_dir = app_data_dir().unwrap();
    let plugins_dir = app_plugins_dir().unwrap();
    let clipboard_img_dir = app_clipboard_img_dir().unwrap();

    let mut map = HashMap::new();
    map.insert("log".to_string(), log_dir.to_str().unwrap().to_string());
    map.insert("data".to_string(), data_dir.to_str().unwrap().to_string());
    map.insert(
        "plugins".to_string(),
        plugins_dir.to_str().unwrap().to_string(),
    );
    map.insert(
        "clipboardImg".to_string(),
        clipboard_img_dir.to_str().unwrap().to_string(),
    );
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("lark-layout-{}-{nonce}", std::process::id()));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn write(&self, path: &str, value: &str) {
            let path = self.0.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, value).unwrap();
        }
        fn read(&self, path: &str) -> String {
            std::fs::read_to_string(self.0.join(path)).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn migrates_old_layout_even_when_installer_has_created_resources() {
        let fixture = Fixture::new();
        fixture.write("config/lark/config.json", "old-config");
        fixture.write("config/lark/data/index.db", "index");
        fixture.write("config/lark/logs/old.log", "log");
        fixture.write("config/lark/data/plugins/user/main.js", "user");
        fixture.write("resources/plugins/bundled/main.js", "bundled");
        assert_eq!(
            initialize_layout(&fixture.0).unwrap(),
            fixture.0.join("resources")
        );
        assert_eq!(fixture.read("resources/config.json"), "old-config");
        assert_eq!(fixture.read("resources/data/index.db"), "index");
        assert_eq!(fixture.read("resources/data/logs/old.log"), "log");
        assert_eq!(fixture.read("resources/plugins/user/main.js"), "user");
        assert!(!fixture.0.join("resources/data/plugins").exists());
        assert_eq!(fixture.read("config/lark/config.json"), "old-config");
    }

    #[test]
    fn preserves_new_data_and_never_merges_conflicting_plugins() {
        let fixture = Fixture::new();
        fixture.write("config/lark/config.json", "old-config");
        fixture.write("resources/config.json", "new-config");
        fixture.write(
            "config/lark/data/plugins/same/manifest.json",
            "old-manifest",
        );
        fixture.write("resources/plugins/same/main.js", "new-entry");
        initialize_layout(&fixture.0).unwrap();
        assert_eq!(fixture.read("resources/config.json"), "new-config");
        assert!(!fixture
            .0
            .join("resources/plugins/same/manifest.json")
            .exists());
        fixture.write("config/lark/data/new-file", "late legacy file");
        initialize_layout(&fixture.0).unwrap();
        assert!(!fixture.0.join("resources/data/new-file").exists());
    }

    #[test]
    fn fresh_layout_does_not_create_legacy_directories() {
        let fixture = Fixture::new();
        initialize_layout(&fixture.0).unwrap();
        assert!(fixture.0.join("resources/data").is_dir());
        assert!(!fixture.0.join("config").exists());
    }

    #[test]
    fn migrates_webview_preferences_only_once() {
        let fixture = Fixture::new();
        fixture.write(
            "old/EBWebView/Default/Local Storage/preferences",
            "disabled-plugin",
        );
        let old = fixture.0.join("old");
        let data = fixture.0.join("resources/data");
        let destination = initialize_webview_data(&data, Some(&old)).unwrap();
        assert_eq!(destination, data.join("webview"));
        assert_eq!(
            fixture.read("resources/data/webview/EBWebView/Default/Local Storage/preferences"),
            "disabled-plugin"
        );
        fixture.write(
            "resources/data/webview/EBWebView/Default/Local Storage/preferences",
            "new-preferences",
        );
        initialize_webview_data(&data, Some(&old)).unwrap();
        assert_eq!(
            fixture.read("resources/data/webview/EBWebView/Default/Local Storage/preferences"),
            "new-preferences"
        );
    }
}
