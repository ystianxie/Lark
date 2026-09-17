// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
mod api;
mod config;
mod utils;

use crate::api::clipboard::{
    get_history_all, get_history_id, get_history_part, get_history_search, ClipboardWatcher,
};
use crate::api::explorer::{
    add_custom_app_index, create_app_index_to_sql, create_file_index_to_sql,
    delete_custom_app_index, get_custom_app_indexes, open_explorer, read_app_info,
    read_file_to_base64, read_icns_to_base64,
};
use crate::api::shell::{
    append_txt, clipboard_control, get_file_icon, open_app, open_file, open_url, read_txt,
    run_python_plugin, run_python_script, write_txt,
};
use crate::config::plugins::{create_plugin, load_plugin_editor, load_plugins, update_plugin};
use crate::config::{
    app_settings, hotkey_settings, index_settings, save_index_settings_data, save_setting_data,
    save_snippet_settings_data, snippet_settings,
};
use crate::utils::database::{FileIndex, IndexSQL, RecordSQL};
use crate::utils::dirs::get_app_dir;
use crate::utils::window::set_window_show;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    App, AppHandle, Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

static HOTKEY_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static HOTKEY_CAPTURE_PAIR: OnceLock<Mutex<Option<(Shortcut, Shortcut)>>> = OnceLock::new();
static HOTKEY_REGISTRATION_LOCK: Mutex<()> = Mutex::new(());

fn hotkey_capture_pair() -> &'static Mutex<Option<(Shortcut, Shortcut)>> {
    HOTKEY_CAPTURE_PAIR.get_or_init(|| Mutex::new(None))
}

#[derive(Clone)]
struct AppState {
    pub app_handle: AppHandle,
}
#[derive(Serialize, Deserialize)]
enum SearchResult {
    Map(HashMap<String, String>),
    File(FileIndex),
}

#[tauri::command(rename_all = "camelCase")]
async fn search_keyword(
    component_name: String,
    input_value: String,
    offset: i32,
    params: HashMap<String, String>,
) -> Result<Vec<SearchResult>, String>
// where
//     T: From<HashMap<String, String>> + From<FileIndex>,
{
    println!(
        "执行搜索 {:?} 关键词 {:?} 参数 {:?}",
        component_name, input_value, params
    );
    let results = tauri::async_runtime::spawn_blocking(move || {
        if component_name.is_empty() {
            return api::explorer::search_app_index(&input_value, offset)
                .into_iter()
                .map(SearchResult::File)
                .collect();
        }
        if component_name == "文件搜索" {
            let result = api::explorer::search_file_index(&input_value, offset);
            println!("文件搜索结果 {:?}", result.len());
            return result.into_iter().map(SearchResult::File).collect();
        }
        Vec::new()
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(results)
}

#[tauri::command]
fn create_file_index(app: AppHandle) {
    let app_handle = app.app_handle().clone();
    static FILE_INDEX_RUNNING: AtomicBool = AtomicBool::new(false);
    if FILE_INDEX_RUNNING.swap(true, Ordering::AcqRel) {
        println!("文件索引任务已在运行");
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        create_file_index_to_sql(app_handle);
        FILE_INDEX_RUNNING.store(false, Ordering::Release);
    });
}

#[tauri::command]
fn create_app_index(app: AppHandle) {
    println!("创建");
    let app_handle = app.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || create_app_index_to_sql(app_handle));
}

#[tauri::command]
fn rebuild_index(app: AppHandle) {
    println!("rebuild index");
    create_file_index(app);
}

fn shortcut(app: &mut App, awaken: &str, clipboard: &str) {
    let awaken_shortcut: Shortcut = awaken.parse().expect("invalid awaken shortcut");
    let clipboard_shortcut: Shortcut = clipboard.parse().expect("invalid clipboard shortcut");
    app.handle()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .unwrap();
    register_shortcuts(app.handle(), awaken_shortcut, clipboard_shortcut)
        .expect("failed to register configured shortcuts");
}

fn parse_shortcut_pair(awaken: &str, clipboard: &str) -> Result<(Shortcut, Shortcut), String> {
    let awaken_shortcut = awaken
        .parse::<Shortcut>()
        .map_err(|error| format!("唤醒快捷键无效：{error}"))?;
    let clipboard_shortcut = clipboard
        .parse::<Shortcut>()
        .map_err(|error| format!("剪贴板快捷键无效：{error}"))?;
    if awaken_shortcut.mods == Modifiers::empty() || clipboard_shortcut.mods == Modifiers::empty() {
        return Err("快捷键必须至少包含一个修饰键（Ctrl、Alt、Shift 或 Win）".to_string());
    }
    if awaken_shortcut == clipboard_shortcut {
        return Err("唤醒快捷键和剪贴板快捷键不能相同".to_string());
    }
    Ok((awaken_shortcut, clipboard_shortcut))
}

fn register_shortcuts(
    app: &AppHandle,
    awaken_shortcut: Shortcut,
    clipboard_shortcut: Shortcut,
) -> Result<(), String> {
    let window = app
        .get_webview_window("skylark")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    let global_shortcut = app.global_shortcut();
    let result = global_shortcut.on_shortcuts(
        [awaken_shortcut, clipboard_shortcut],
        move |_app, pressed, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            if *pressed == awaken_shortcut {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.emit("window-show-request", ());
                }
            } else if *pressed == clipboard_shortcut {
                let _ = window.emit("clipboard-show-request", ());
            }
        },
    );
    if let Err(error) = result {
        let _ = global_shortcut.unregister_all();
        return Err(error.to_string());
    }
    Ok(())
}

fn capture_shortcut_set(awaken: Shortcut, clipboard: Shortcut) -> Vec<Shortcut> {
    let mut shortcuts = vec![awaken, clipboard, "Alt+Space".parse().unwrap()];
    shortcuts.sort_by_key(|shortcut| shortcut.id());
    shortcuts.dedup();
    shortcuts
}

fn register_capture_shortcuts(
    app: &AppHandle,
    awaken_shortcut: Shortcut,
    clipboard_shortcut: Shortcut,
) -> Result<(), String> {
    let window = app
        .get_webview_window("skylark")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    let alt_space: Shortcut = "Alt+Space".parse().unwrap();
    let shortcuts = capture_shortcut_set(awaken_shortcut, clipboard_shortcut);
    let global_shortcut = app.global_shortcut();
    let result = global_shortcut.on_shortcuts(shortcuts, move |_app, pressed, event| {
        if event.state == ShortcutState::Pressed && *pressed == alt_space {
            let _ = window.emit("hotkey-capture", pressed.to_string());
        }
    });
    if let Err(error) = result {
        let _ = global_shortcut.unregister_all();
        return Err(error.to_string());
    }
    Ok(())
}

fn restore_shortcuts(
    app: &AppHandle,
    awaken_shortcut: Shortcut,
    clipboard_shortcut: Shortcut,
) -> Result<(), String> {
    let global_shortcut = app.global_shortcut();
    global_shortcut
        .unregister_all()
        .map_err(|error| format!("清理现有快捷键失败：{error}"))?;
    register_shortcuts(app, awaken_shortcut, clipboard_shortcut)
}

fn restore_capture_shortcuts(
    app: &AppHandle,
    awaken_shortcut: Shortcut,
    clipboard_shortcut: Shortcut,
) -> Result<(), String> {
    let global_shortcut = app.global_shortcut();
    global_shortcut
        .unregister_all()
        .map_err(|error| format!("清理现有快捷键失败：{error}"))?;
    register_capture_shortcuts(app, awaken_shortcut, clipboard_shortcut)
}

fn clear_hotkey_capture_state() {
    HOTKEY_CAPTURE_ACTIVE.store(false, Ordering::Release);
    *hotkey_capture_pair().lock().unwrap() = None;
}

#[tauri::command(rename_all = "camelCase")]
fn save_setting(app: AppHandle, setting_info: serde_json::Value) -> Result<(), String> {
    let _registration_guard = HOTKEY_REGISTRATION_LOCK.lock().unwrap();
    let (old_awaken, old_clipboard) = hotkey_settings().map_err(|error| error.to_string())?;
    let awaken = setting_info
        .get("hotkeyAwaken")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&old_awaken);
    let clipboard = setting_info
        .get("hotkeyClipboard")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&old_clipboard);
    let (awaken_shortcut, clipboard_shortcut) = parse_shortcut_pair(awaken, clipboard)?;
    let (old_awaken_shortcut, old_clipboard_shortcut) =
        parse_shortcut_pair(&old_awaken, &old_clipboard)?;

    if awaken_shortcut == old_awaken_shortcut
        && clipboard_shortcut == old_clipboard_shortcut
        && !HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire)
    {
        save_setting_data(setting_info).map_err(|error| error.to_string())?;
        return Ok(());
    }

    let global_shortcut = app.global_shortcut();
    if let Err(error) = global_shortcut.unregister_all() {
        let rollback = restore_shortcuts(&app, old_awaken_shortcut, old_clipboard_shortcut);
        if rollback.is_ok() {
            clear_hotkey_capture_state();
        }
        return Err(match rollback {
            Ok(()) => format!("旧快捷键注销失败，已恢复原快捷键：{error}"),
            Err(rollback_error) => {
                format!("旧快捷键注销失败，恢复原快捷键也失败：{error}；{rollback_error}")
            }
        });
    }
    if let Err(error) = register_shortcuts(&app, awaken_shortcut, clipboard_shortcut) {
        let rollback = restore_shortcuts(&app, old_awaken_shortcut, old_clipboard_shortcut);
        if rollback.is_ok() {
            clear_hotkey_capture_state();
        }
        return Err(match rollback {
            Ok(()) => format!("新快捷键注册失败，已恢复原快捷键：{error}"),
            Err(rollback_error) => {
                format!("新快捷键注册失败，恢复原快捷键也失败：{error}；{rollback_error}")
            }
        });
    }

    if let Err(error) = save_setting_data(setting_info) {
        let rollback = restore_shortcuts(&app, old_awaken_shortcut, old_clipboard_shortcut);
        if rollback.is_ok() {
            clear_hotkey_capture_state();
        }
        return Err(match rollback {
            Ok(()) => format!("设置写入失败，已恢复原快捷键：{error}"),
            Err(rollback_error) => {
                format!("设置写入失败，恢复原快捷键也失败：{error}；{rollback_error}")
            }
        });
    }
    clear_hotkey_capture_state();
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn set_hotkey_capture_active(app: AppHandle, active: bool) -> Result<(), String> {
    let _registration_guard = HOTKEY_REGISTRATION_LOCK.lock().unwrap();
    if active == HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire) {
        return Ok(());
    }
    let (awaken, clipboard) = hotkey_settings().map_err(|error| error.to_string())?;
    let (awaken_shortcut, clipboard_shortcut) = parse_shortcut_pair(&awaken, &clipboard)?;
    let previous_capture_pair = *hotkey_capture_pair().lock().unwrap();
    let global_shortcut = app.global_shortcut();
    if let Err(error) = global_shortcut.unregister_all() {
        let rollback = if active {
            restore_shortcuts(&app, awaken_shortcut, clipboard_shortcut)
        } else if let Some((previous_awaken, previous_clipboard)) = previous_capture_pair {
            restore_capture_shortcuts(&app, previous_awaken, previous_clipboard)
        } else {
            restore_capture_shortcuts(&app, awaken_shortcut, clipboard_shortcut)
        };
        return Err(match rollback {
            Ok(()) => format!("注销现有快捷键失败，已恢复原快捷键：{error}"),
            Err(rollback_error) => {
                format!("注销现有快捷键失败，恢复原快捷键也失败：{error}；{rollback_error}")
            }
        });
    }

    let registration = if active {
        register_capture_shortcuts(&app, awaken_shortcut, clipboard_shortcut)
    } else {
        register_shortcuts(&app, awaken_shortcut, clipboard_shortcut)
    };
    if let Err(error) = registration {
        let rollback = if active {
            restore_shortcuts(&app, awaken_shortcut, clipboard_shortcut)
        } else if let Some((previous_awaken, previous_clipboard)) = previous_capture_pair {
            restore_capture_shortcuts(&app, previous_awaken, previous_clipboard)
        } else {
            restore_capture_shortcuts(&app, awaken_shortcut, clipboard_shortcut)
        };
        return Err(match rollback {
            Ok(()) => format!("快捷键录入模式切换失败，已恢复原快捷键：{error}"),
            Err(rollback_error) => {
                format!("快捷键录入模式切换失败，恢复原快捷键也失败：{error}；{rollback_error}")
            }
        });
    }
    HOTKEY_CAPTURE_ACTIVE.store(active, Ordering::Release);
    *hotkey_capture_pair().lock().unwrap() = if active {
        Some((awaken_shortcut, clipboard_shortcut))
    } else {
        None
    };
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn reserve_hotkey_capture(app: AppHandle, awaken: String, clipboard: String) -> Result<(), String> {
    let _registration_guard = HOTKEY_REGISTRATION_LOCK.lock().unwrap();
    if !HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire) {
        return Err("快捷键录入模式未开启".to_string());
    }
    let (awaken_shortcut, clipboard_shortcut) = parse_shortcut_pair(&awaken, &clipboard)?;
    let previous_pair = hotkey_capture_pair()
        .lock()
        .unwrap()
        .ok_or_else(|| "快捷键录入状态已失效".to_string())?;
    let global_shortcut = app.global_shortcut();
    if let Err(error) = global_shortcut.unregister_all() {
        let rollback = restore_capture_shortcuts(&app, previous_pair.0, previous_pair.1);
        return Err(match rollback {
            Ok(()) => format!("更新录入占位快捷键失败，已恢复原录入快捷键：{error}"),
            Err(rollback_error) => {
                format!("更新录入占位快捷键失败，恢复原录入快捷键也失败：{error}；{rollback_error}")
            }
        });
    }
    if let Err(error) = register_capture_shortcuts(&app, awaken_shortcut, clipboard_shortcut) {
        let rollback = restore_capture_shortcuts(&app, previous_pair.0, previous_pair.1);
        return Err(match rollback {
            Ok(()) => format!("快捷键暂时无法占用，已恢复原录入快捷键：{error}"),
            Err(rollback_error) => {
                format!("快捷键暂时无法占用，恢复原录入快捷键也失败：{error}；{rollback_error}")
            }
        });
    }
    *hotkey_capture_pair().lock().unwrap() = Some((awaken_shortcut, clipboard_shortcut));
    Ok(())
}

#[tauri::command]
fn get_hotkey_settings() -> Result<serde_json::Value, String> {
    let (awaken, clipboard) = hotkey_settings().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({ "hotkeyAwaken": awaken, "hotkeyClipboard": clipboard }))
}

#[tauri::command]
fn get_app_settings() -> Result<serde_json::Value, String> {
    app_settings().map_err(|error| error.to_string())
}

#[tauri::command]
fn trigger_listary_jump(
    state: tauri::State<'_, api::listary_jump::ListaryJumpHandle>,
) -> Result<String, String> {
    state.trigger_ctrl_g().map(|method| format!("{method:?}"))
}

#[tauri::command]
fn get_index_settings() -> Result<serde_json::Value, String> {
    index_settings().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_snippet_settings() -> Result<serde_json::Value, String> {
    snippet_settings().map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn save_snippet_settings(setting_info: serde_json::Value) -> Result<(), String> {
    let (enabled, trigger, snippets) =
        save_snippet_settings_data(setting_info).map_err(|error| error.to_string())?;
    api::snippets::update_settings(enabled, trigger, snippets);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn save_index_settings(setting_info: serde_json::Value) -> Result<(), String> {
    save_index_settings_data(setting_info).map_err(|error| error.to_string())
}

fn main() {
    ClipboardWatcher::start();
    IndexSQL::new();
    RecordSQL::new();
    let config = config::Config::read_local_config().unwrap();
    let hotkey_awaken = config.base.hotkey_awaken.clone();
    let hotkey_clipboard = config.base.hotkey_clipboard.clone();
    api::snippets::update_settings(
        config.base.snippets_enabled,
        config.base.snippet_trigger.clone(),
        config.base.text_snippets.clone(),
    );
    api::snippets::start();

    tauri::Builder::default()
        .setup(move |app| {
            app.manage(api::listary_jump::ListaryJumpHandle::start());
            let plugins_dir = crate::utils::dirs::app_plugins_dir()?;
            app.asset_protocol_scope()
                .allow_directory(plugins_dir, true)?;
            shortcut(app, &hotkey_awaken, &hotkey_clipboard);
            utils::window::disable_system_menu(app)?;
            utils::window::set_window_shadow(app);
            println!("{:?}", &hotkey_awaken);
            let main_window = app.get_window("skylark").unwrap();
            let position = main_window.outer_position().unwrap();
            println!("{:?}", position);

            // The tray icon itself is created from `tauri.conf.json`; attach its
            // context menu here so the user can terminate the application.
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;
            if let Some(tray) = app.tray_by_id("main") {
                tray.set_menu(Some(tray_menu))?;
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            search_keyword,
            create_file_index,
            create_app_index,
            rebuild_index,
            open_app,
            open_url,
            get_file_icon,
            run_python_script,
            run_python_plugin,
            clipboard_control,
            write_txt,
            read_txt,
            append_txt,
            open_file,
            read_app_info,
            open_explorer,
            read_file_to_base64,
            read_icns_to_base64,
            set_window_show,
            get_history_all,
            get_history_id,
            get_history_part,
            get_history_search,
            load_plugins,
            create_plugin,
            load_plugin_editor,
            update_plugin,
            get_app_dir,
            save_setting,
            set_hotkey_capture_active,
            reserve_hotkey_capture,
            get_hotkey_settings,
            trigger_listary_jump,
            get_app_settings,
            get_index_settings,
            save_index_settings,
            get_snippet_settings,
            save_snippet_settings,
            add_custom_app_index,
            get_custom_app_indexes,
            delete_custom_app_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{capture_shortcut_set, parse_shortcut_pair};

    #[test]
    fn shortcut_pair_accepts_distinct_modified_shortcuts() {
        assert!(parse_shortcut_pair("Alt+Space", "Shift+Alt+V").is_ok());
    }

    #[test]
    fn shortcut_pair_rejects_duplicate_shortcuts() {
        assert!(parse_shortcut_pair("Alt+Space", "Alt+Space")
            .unwrap_err()
            .contains("不能相同"));
    }

    #[test]
    fn shortcut_pair_rejects_unmodified_keys() {
        assert!(parse_shortcut_pair("A", "Shift+Alt+V")
            .unwrap_err()
            .contains("修饰键"));
    }

    #[test]
    fn capture_shortcuts_always_include_alt_space_without_duplicates() {
        let (awaken, clipboard) = parse_shortcut_pair("Alt+Space", "Shift+Alt+V").unwrap();
        let shortcuts = capture_shortcut_set(awaken, clipboard);
        assert_eq!(shortcuts.len(), 2);
        assert!(shortcuts.contains(&"Alt+Space".parse().unwrap()));
    }
}
