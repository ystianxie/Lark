use anyhow::Result;
use std::path::PathBuf;
use tauri::api::path::home_dir;

static APP_DIR: &str = "lark";
static CONFIG_FILE: &str = "config.json";

/// get the app home dir
pub fn app_home_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use tauri::utils::platform::current_exe;

        let app_exe = current_exe()?;
        let app_exe = dunce::canonicalize(app_exe)?;
        let app_dir = app_exe
            .parent()
            .ok_or(anyhow::anyhow!("failed to get the portable app dir"))?;
        let app_dir = PathBuf::from(app_dir).join(".config").join(APP_DIR);
        if !app_dir.exists() {
            std::fs::create_dir_all(&app_dir)?;
        }
        Ok(app_dir)
    }

    #[cfg(not(target_os = "windows"))]
    let home = home_dir()
        .ok_or(anyhow::anyhow!("failed to get the app home dir"))?
        .join(".config")
        .join(APP_DIR);
    if !home.exists() {
        std::fs::create_dir_all(&home)?;
    }
    Ok(home)
}

/// logs dir
#[allow(unused)]
pub fn app_logs_dir() -> Result<PathBuf> {
    let logs = app_home_dir()?.join("logs");
    if !logs.exists() {
        std::fs::create_dir(&logs)?;
    }
    Ok(logs)
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(CONFIG_FILE))
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
    let plugins = app_data_dir()?.join("plugins");
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

#[test]
fn test() {
    println!("app_home_dir: {:?}", app_home_dir());
    println!("app_logs_dir: {:?}", app_logs_dir());
    println!("config_path: {:?}", config_path());
    println!("app_data_dir: {:?}", app_data_dir());
    println!("app_plugins_dir: {:?}", app_plugins_dir());
    println!("app_clipboard_img_dir: {:?}", app_clipboard_img_dir());
}
