// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::collections::HashMap;
use std::path::PathBuf;
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
    append_txt, clipboard_control, get_file_icon, open_app, open_file, open_url,
    probe_python_interpreter, read_txt, run_python_plugin, run_python_script, write_txt,
};
use crate::config::plugins::{
    create_plugin, load_plugin_editor, load_plugins, update_plugin, valid_plugin_id,
};
use crate::config::{
    app_settings, clear_plugin_settings_data, hotkey_settings, index_initialization_flags_present,
    index_settings, plugin_settings, plugin_settings_map, save_index_initialization_flags,
    save_index_settings_data, save_plugin_settings_data, save_setting_data,
    save_snippet_settings_data, snippet_settings,
};
use crate::utils::database::{FileIndex, IndexSQL, RecordSQL};
use crate::utils::dirs::get_app_dir;
use crate::utils::window::set_window_show;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconEvent},
    App, AppHandle, Emitter, Manager,
};
use auto_launch::AutoLaunchBuilder;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

static HOTKEY_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static HOTKEY_CAPTURE_BINDINGS: OnceLock<Mutex<Option<HotkeyBindings>>> = OnceLock::new();
static HOTKEY_REGISTRATION_LOCK: Mutex<()> = Mutex::new(());
static INDEX_SETTINGS_LOCK: Mutex<()> = Mutex::new(());
static FILE_INDEX_RUNNING: AtomicBool = AtomicBool::new(false);

fn hotkey_capture_bindings() -> &'static Mutex<Option<HotkeyBindings>> {
    HOTKEY_CAPTURE_BINDINGS.get_or_init(|| Mutex::new(None))
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
    if FILE_INDEX_RUNNING.swap(true, Ordering::AcqRel) {
        println!("文件索引任务已在运行");
        return;
    }
    let _ = app
        .state::<Mutex<api::file_watcher::FileIndexUpdateService>>()
        .lock()
        .unwrap()
        .begin_rebuild();
    let app_handle = app.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = create_file_index_to_sql(app_handle.clone()) {
            eprintln!("文件索引重建失败: {error}");
        }
        if let Some(service) =
            app_handle.try_state::<Mutex<api::file_watcher::FileIndexUpdateService>>()
        {
            let _ = service.lock().unwrap().finish_rebuild();
        }
        FILE_INDEX_RUNNING.store(false, Ordering::Release);
    });
}

#[tauri::command]
fn create_app_index(app: AppHandle) {
    println!("创建应用索引");
    #[cfg(target_os = "windows")]
    {
        let submitted = {
            let watcher_state =
                app.state::<Mutex<Option<api::windows_app_watcher::WindowsAppWatcher>>>();
            let watcher = watcher_state.lock().unwrap();
            if let Some(watcher) = watcher.as_ref() {
                watcher.request_refresh();
                true
            } else {
                false
            }
        };
        if !submitted {
            let app_handle = app.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = create_app_index_to_sql(app_handle) {
                    eprintln!("应用索引重建失败: {error}");
                }
            });
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let app_handle = app.app_handle().clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) = create_app_index_to_sql(app_handle) {
                eprintln!("应用索引重建失败: {error}");
            }
        });
    }
}

#[tauri::command]
fn rebuild_index(app: AppHandle) {
    println!("rebuild index");
    create_file_index(app);
}

fn shortcut(app: &mut App, awaken: &str, clipboard: &str, file_jump: &str) {
    let bindings = parse_shortcut_bindings(awaken, clipboard, file_jump)
        .expect("invalid configured shortcuts");
    app.handle()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .unwrap();
    register_shortcuts(app.handle(), bindings).expect("failed to register configured shortcuts");
}

#[derive(Clone, Copy)]
struct HotkeyBindings {
    awaken: Shortcut,
    clipboard: Shortcut,
    file_jump: Shortcut,
}

impl HotkeyBindings {
    fn parse(awaken: &str, clipboard: &str, file_jump: &str) -> Result<Self, String> {
        let awaken = awaken
            .parse::<Shortcut>()
            .map_err(|error| format!("唤醒快捷键无效：{error}"))?;
        let clipboard = clipboard
            .parse::<Shortcut>()
            .map_err(|error| format!("剪贴板快捷键无效：{error}"))?;
        let file_jump = file_jump
            .parse::<Shortcut>()
            .map_err(|error| format!("文件跳转快捷键无效：{error}"))?;
        if [awaken, clipboard, file_jump]
            .iter()
            .any(|shortcut| shortcut.mods == Modifiers::empty())
        {
            return Err("快捷键必须至少包含一个修饰键（Ctrl、Alt、Shift 或 Win）".to_string());
        }
        if awaken == clipboard || awaken == file_jump || clipboard == file_jump {
            return Err("唤醒、剪贴板和文件跳转快捷键不能相同".to_string());
        }
        Ok(Self {
            awaken,
            clipboard,
            file_jump,
        })
    }

    fn all(self) -> [Shortcut; 3] {
        [self.awaken, self.clipboard, self.file_jump]
    }
}

fn parse_shortcut_bindings(
    awaken: &str,
    clipboard: &str,
    file_jump: &str,
) -> Result<HotkeyBindings, String> {
    HotkeyBindings::parse(awaken, clipboard, file_jump)
}

fn register_shortcuts(app: &AppHandle, bindings: HotkeyBindings) -> Result<(), String> {
    let window = app
        .get_webview_window("skylark")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    let global_shortcut = app.global_shortcut();
    let result = global_shortcut.on_shortcuts(bindings.all(), move |app, pressed, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        if *pressed == bindings.awaken {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.emit("window-show-request", ());
            }
        } else if *pressed == bindings.clipboard {
            let _ = window.emit("clipboard-show-request", ());
        } else if *pressed == bindings.file_jump {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = app
                    .state::<api::listary_jump::ListaryJumpHandle>()
                    .trigger_ctrl_g()
                {
                    eprintln!("[ListaryJump] 快捷键触发失败：{error}");
                }
            });
        }
    });
    if let Err(error) = result {
        let _ = global_shortcut.unregister_all();
        return Err(error.to_string());
    }
    Ok(())
}

fn capture_shortcut_set(bindings: HotkeyBindings) -> Vec<Shortcut> {
    let mut shortcuts = vec![
        bindings.awaken,
        bindings.clipboard,
        bindings.file_jump,
        "Alt+Space".parse().unwrap(),
    ];
    shortcuts.sort_by_key(|shortcut| shortcut.id());
    shortcuts.dedup();
    shortcuts
}

fn register_capture_shortcuts(app: &AppHandle, bindings: HotkeyBindings) -> Result<(), String> {
    let window = app
        .get_webview_window("skylark")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    let alt_space: Shortcut = "Alt+Space".parse().unwrap();
    let global_shortcut = app.global_shortcut();
    let result = global_shortcut.on_shortcuts(
        capture_shortcut_set(bindings),
        move |_app, pressed, event| {
            if event.state == ShortcutState::Pressed && *pressed == alt_space {
                let _ = window.emit("hotkey-capture", pressed.to_string());
            }
        },
    );
    if let Err(error) = result {
        let _ = global_shortcut.unregister_all();
        return Err(error.to_string());
    }
    Ok(())
}

fn replace_shortcuts<F>(app: &AppHandle, register: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    let global_shortcut = app.global_shortcut();
    global_shortcut
        .unregister_all()
        .map_err(|error| format!("清理现有快捷键失败：{error}"))?;
    register()
}

fn restore_shortcuts(app: &AppHandle, bindings: HotkeyBindings) -> Result<(), String> {
    replace_shortcuts(app, || register_shortcuts(app, bindings))
}

fn restore_capture_shortcuts(app: &AppHandle, bindings: HotkeyBindings) -> Result<(), String> {
    replace_shortcuts(app, || register_capture_shortcuts(app, bindings))
}

fn clear_hotkey_capture_state() {
    HOTKEY_CAPTURE_ACTIVE.store(false, Ordering::Release);
    *hotkey_capture_bindings().lock().unwrap() = None;
}

#[tauri::command(rename_all = "camelCase")]
fn save_setting(app: AppHandle, setting_info: serde_json::Value) -> Result<(), String> {
    let _registration_guard = HOTKEY_REGISTRATION_LOCK.lock().unwrap();
    let (old_awaken, old_clipboard, old_file_jump) =
        hotkey_settings().map_err(|error| error.to_string())?;
    let awaken = setting_info
        .get("hotkeyAwaken")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&old_awaken);
    let clipboard = setting_info
        .get("hotkeyClipboard")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&old_clipboard);
    let file_jump = setting_info
        .get("hotkeyFileJump")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&old_file_jump);
    let bindings = parse_shortcut_bindings(awaken, clipboard, file_jump)?;
    let old_bindings = parse_shortcut_bindings(&old_awaken, &old_clipboard, &old_file_jump)?;
    if bindings.awaken == old_bindings.awaken
        && bindings.clipboard == old_bindings.clipboard
        && bindings.file_jump == old_bindings.file_jump
        && !HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire)
    {
        save_setting_data(setting_info).map_err(|error| error.to_string())?;
        return Ok(());
    }
    if let Err(error) = replace_shortcuts(&app, || register_shortcuts(&app, bindings)) {
        let rollback = restore_shortcuts(&app, old_bindings);
        return Err(match rollback {
            Ok(()) => format!("新快捷键注册失败，已恢复原快捷键：{error}"),
            Err(e) => format!("新快捷键注册失败，恢复原快捷键也失败：{error}；{e}"),
        });
    }
    if let Err(error) = save_setting_data(setting_info) {
        let rollback = restore_shortcuts(&app, old_bindings);
        return Err(match rollback {
            Ok(()) => format!("设置写入失败，已恢复原快捷键：{error}"),
            Err(e) => format!("设置写入失败，恢复原快捷键也失败：{error}；{e}"),
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
    let (awaken, clipboard, file_jump) = hotkey_settings().map_err(|error| error.to_string())?;
    let bindings = parse_shortcut_bindings(&awaken, &clipboard, &file_jump)?;
    let previous = *hotkey_capture_bindings().lock().unwrap();
    let registration = if active {
        replace_shortcuts(&app, || register_capture_shortcuts(&app, bindings))
    } else {
        replace_shortcuts(&app, || register_shortcuts(&app, bindings))
    };
    if let Err(error) = registration {
        let rollback = if active {
            restore_shortcuts(&app, bindings)
        } else if let Some(previous) = previous {
            restore_capture_shortcuts(&app, previous)
        } else {
            restore_shortcuts(&app, bindings)
        };
        return Err(match rollback {
            Ok(()) => format!("快捷键录入模式切换失败，已恢复原快捷键：{error}"),
            Err(e) => format!("快捷键录入模式切换失败，恢复原快捷键也失败：{error}；{e}"),
        });
    }
    HOTKEY_CAPTURE_ACTIVE.store(active, Ordering::Release);
    *hotkey_capture_bindings().lock().unwrap() = active.then_some(bindings);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn reserve_hotkey_capture(
    app: AppHandle,
    awaken: String,
    clipboard: String,
    file_jump: String,
) -> Result<(), String> {
    let _registration_guard = HOTKEY_REGISTRATION_LOCK.lock().unwrap();
    if !HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire) {
        return Err("快捷键录入模式未开启".to_string());
    }
    let bindings = parse_shortcut_bindings(&awaken, &clipboard, &file_jump)?;
    let previous = hotkey_capture_bindings()
        .lock()
        .unwrap()
        .ok_or_else(|| "快捷键录入状态已失效".to_string())?;
    if let Err(error) = replace_shortcuts(&app, || register_capture_shortcuts(&app, bindings)) {
        let rollback = restore_capture_shortcuts(&app, previous);
        return Err(match rollback {
            Ok(()) => format!("快捷键暂时无法占用，已恢复原录入快捷键：{error}"),
            Err(e) => format!("快捷键暂时无法占用，恢复原录入快捷键也失败：{error}；{e}"),
        });
    }
    *hotkey_capture_bindings().lock().unwrap() = Some(bindings);
    Ok(())
}

#[tauri::command]
fn get_hotkey_settings() -> Result<serde_json::Value, String> {
    let (awaken, clipboard, file_jump) = hotkey_settings().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "hotkeyAwaken": awaken,
        "hotkeyClipboard": clipboard,
        "hotkeyFileJump": file_jump,
    }))
}

#[tauri::command]
fn get_app_settings() -> Result<serde_json::Value, String> {
    app_settings().map_err(|error| error.to_string())
}

fn auto_launch_instance() -> Result<auto_launch::AutoLaunch, String> {
    let path = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name("Lark").set_app_path(path.to_string_lossy().as_ref());
    builder.build().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_auto_launch_enabled() -> Result<bool, String> {
    auto_launch_instance()?.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_auto_launch_enabled(enabled: bool) -> Result<(), String> {
    let auto = auto_launch_instance()?;
    if enabled { auto.enable() } else { auto.disable() }.map_err(|e| e.to_string())
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
fn save_index_settings(app: AppHandle, setting_info: serde_json::Value) -> Result<(), String> {
    let _settings_guard = INDEX_SETTINGS_LOCK
        .lock()
        .map_err(|_| "索引设置锁异常".to_string())?;
    let before = config::Config::read_local_config().unwrap_or_default();
    save_index_settings_data(setting_info).map_err(|error| error.to_string())?;
    let after = config::Config::read_local_config().unwrap_or_default();
    // 配置差异和 watcher 事件必须串行化：先暂停旧 watcher，再提交局部索引批次。
    stop_file_watcher(&app);
    apply_local_index_settings_delta(&app, &before.base, &after.base);
    start_file_watcher(&app);
    Ok(())
}

#[tauri::command]
fn get_file_index_status(app: AppHandle) -> String {
    app.state::<Mutex<api::file_watcher::FileIndexUpdateService>>()
        .lock()
        .unwrap()
        .status()
        .to_string()
}

fn apply_local_index_settings_delta(
    app: &AppHandle,
    before: &config::BaseConfig,
    after: &config::BaseConfig,
) {
    let old_roots = before.local_file_search_paths.clone().unwrap_or_default();
    let new_roots = after.local_file_search_paths.clone().unwrap_or_default();
    let old_excludes = &before.local_file_search_exclude_paths;
    let new_excludes = &after.local_file_search_exclude_paths;
    let mut changes = Vec::new();
    for root in new_roots
        .iter()
        .filter(|root| !old_roots.iter().any(|old| same_config_path(old, root)))
    {
        collect_local_index_changes(PathBuf::from(root).as_path(), after, &mut changes);
    }
    for root in old_roots.iter().filter(|root| {
        !new_roots
            .iter()
            .any(|new_root| same_config_path(new_root, root))
            && !new_roots.iter().any(|remaining| {
                api::file_watcher::path_is_under(
                    PathBuf::from(root).as_path(),
                    PathBuf::from(remaining).as_path(),
                )
            })
    }) {
        changes.push(api::file_watcher::IndexMessage::Batch(vec![
            crate::utils::database::FileIndexChange::Remove {
                path: root.clone(),
                recursive: true,
            },
        ]));
    }
    for path in new_excludes
        .iter()
        .filter(|path| !old_excludes.iter().any(|old| same_config_path(old, path)))
    {
        if !new_roots.iter().any(|root| {
            api::file_watcher::path_is_under(
                PathBuf::from(path).as_path(),
                PathBuf::from(root).as_path(),
            )
        }) {
            continue;
        }
        changes.push(api::file_watcher::IndexMessage::Batch(vec![
            crate::utils::database::FileIndexChange::Remove {
                path: path.clone(),
                recursive: true,
            },
        ]));
    }
    for extension in after
        .local_file_search_exclude_types
        .iter()
        .filter(|ext| !before.local_file_search_exclude_types.contains(ext))
    {
        changes.push(api::file_watcher::IndexMessage::Batch(vec![
            crate::utils::database::FileIndexChange::RemoveByType {
                file_type: extension
                    .trim()
                    .trim_start_matches('.')
                    .to_ascii_lowercase(),
                roots: new_roots.clone(),
            },
        ]));
    }
    // 取消排除目录或扩展名时，只补扫受影响的路径；扫描仍应用最终的全部规则。
    for path in old_excludes
        .iter()
        .filter(|path| !new_excludes.iter().any(|new| same_config_path(new, path)))
    {
        let target = PathBuf::from(path);
        if new_roots
            .iter()
            .any(|root| api::file_watcher::path_is_under(&target, PathBuf::from(root).as_path()))
        {
            collect_local_index_changes(&target, after, &mut changes);
        }
    }
    for extension in before
        .local_file_search_exclude_types
        .iter()
        .filter(|ext| !after.local_file_search_exclude_types.contains(ext))
    {
        let normalized = extension
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let indexes = new_roots
            .iter()
            .flat_map(|root| {
                let watch_config = api::file_watcher::WatchConfig {
                    roots: vec![],
                    excluded_paths: after
                        .local_file_search_exclude_paths
                        .iter()
                        .map(PathBuf::from)
                        .collect(),
                    excluded_extensions: after.local_file_search_exclude_types.clone(),
                    ..Default::default()
                };
                walkdir::WalkDir::new(root)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|entry| !api::file_watcher::is_excluded(entry.path(), &watch_config))
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case(&normalized))
                    })
                    .filter_map(|entry| api::explorer::file_path_to_index(entry.path()))
                    .map(crate::utils::database::FileIndexChange::Upsert)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if !indexes.is_empty() {
            changes.push(api::file_watcher::IndexMessage::Batch(indexes));
        }
    }
    if let Some(state) = app.try_state::<Mutex<api::file_watcher::FileIndexUpdateService>>() {
        let service = state.lock().unwrap();
        if let Err(error) = service.apply_batches_sync(
            changes
                .into_iter()
                .filter_map(|message| match message {
                    api::file_watcher::IndexMessage::Batch(batch) => Some(batch),
                    _ => None,
                })
                .collect(),
        ) {
            eprintln!("[FileWatcher] 配置差异索引写入失败: {error}");
        }
    }
}

fn collect_local_index_changes(
    root: &std::path::Path,
    config: &config::BaseConfig,
    changes: &mut Vec<api::file_watcher::IndexMessage>,
) {
    if !root.is_dir() {
        return;
    }
    let watch_config = api::file_watcher::WatchConfig {
        roots: vec![],
        excluded_paths: config
            .local_file_search_exclude_paths
            .iter()
            .map(PathBuf::from)
            .collect(),
        excluded_extensions: config.local_file_search_exclude_types.clone(),
        ..Default::default()
    };
    let indexes = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| !api::file_watcher::is_excluded(entry.path(), &watch_config))
        .filter_map(|entry| api::explorer::file_path_to_index(entry.path()))
        .map(crate::utils::database::FileIndexChange::Upsert)
        .collect::<Vec<_>>();
    if !indexes.is_empty() {
        changes.push(api::file_watcher::IndexMessage::Batch(indexes));
    }
}

fn same_config_path(left: &str, right: &str) -> bool {
    let normalize = |value: &str| {
        let mut value = value.trim().replace('/', "\\");
        while value.ends_with('\\') && value.len() > 3 {
            value.pop();
        }
        value
            .strip_prefix(r"\\?\")
            .unwrap_or(&value)
            .to_ascii_lowercase()
    };
    normalize(left) == normalize(right)
}

/// 单个插件的配置序列化上限，避免 config.json 被当作任意数据仓库。
const PLUGIN_SETTINGS_MAX_BYTES: usize = 64 * 1024;

/// 读取插件声明过的配置项名称，并要求插件 id 合法且确实已被发现。
fn plugin_config_keys(app: &AppHandle, plugin_id: &str) -> Result<Vec<String>, String> {
    if !valid_plugin_id(plugin_id) {
        return Err("插件 ID 无效或为系统保留名称".into());
    }
    let record = load_plugins(app.clone())
        .into_iter()
        .find(|plugin| plugin.id.as_str() == plugin_id)
        .ok_or_else(|| format!("未找到插件：{plugin_id}"))?;
    if let Some(error) = record.error {
        return Err(format!("插件配置异常：{error}"));
    }
    Ok(record.manifest["config"]["fields"]
        .as_array()
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| field["key"].as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

/// 只保留插件声明过的配置项，防止 config.json 被写入任意键。
fn select_plugin_settings(values: serde_json::Value, keys: &[String]) -> serde_json::Value {
    let mut selected = serde_json::Map::new();
    if let Some(object) = values.as_object() {
        for key in keys {
            if let Some(value) = object.get(key) {
                selected.insert(key.clone(), value.clone());
            }
        }
    }
    serde_json::Value::Object(selected)
}

#[tauri::command(rename_all = "camelCase")]
fn get_plugin_settings(app: AppHandle, plugin_id: String) -> Result<serde_json::Value, String> {
    let keys = plugin_config_keys(&app, &plugin_id)?;
    let stored = plugin_settings(&plugin_id).map_err(|error| error.to_string())?;
    Ok(select_plugin_settings(stored, &keys))
}

#[tauri::command(rename_all = "camelCase")]
fn save_plugin_settings(
    app: AppHandle,
    plugin_id: String,
    values: serde_json::Value,
) -> Result<(), String> {
    let keys = plugin_config_keys(&app, &plugin_id)?;
    let filtered = select_plugin_settings(values, &keys);
    if filtered
        .as_object()
        .map_or(true, |object| object.is_empty())
    {
        return Err("没有可保存的配置项，请确认配置项名称与插件声明一致".into());
    }
    let encoded = serde_json::to_string(&filtered).map_err(|error| error.to_string())?;
    if encoded.len() > PLUGIN_SETTINGS_MAX_BYTES {
        return Err("配置内容过大，请精简后重试".into());
    }
    save_plugin_settings_data(&plugin_id, filtered).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn clear_plugin_settings(app: AppHandle, plugin_id: String) -> Result<(), String> {
    plugin_config_keys(&app, &plugin_id)?;
    clear_plugin_settings_data(&plugin_id).map_err(|error| error.to_string())
}

/// 空值判定与前端保持一致：false 和 0 是有意义的取值，只有 null 与空字符串算未填写。
fn is_empty_config_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(text) => text.trim().is_empty(),
        _ => false,
    }
}

/// 只返回「必填项是否齐全」的摘要，不回传任何配置值，避免把插件密钥读进前端内存。
#[tauri::command]
fn get_plugin_settings_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let stored = plugin_settings_map().map_err(|error| error.to_string())?;
    let mut status = serde_json::Map::new();
    for record in load_plugins(app) {
        if record.error.is_some() {
            continue;
        }
        let Some(fields) = record.manifest["config"]["fields"].as_array() else {
            continue;
        };
        let values = stored.get(&record.id);
        let mut missing = Vec::new();
        for field in fields {
            if field.get("required").and_then(serde_json::Value::as_bool) != Some(true) {
                continue;
            }
            let Some(key) = field.get("key").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let filled = values
                .and_then(|value| value.get(key))
                .is_some_and(|value| !is_empty_config_value(value));
            if !filled {
                missing.push(serde_json::Value::String(key.to_string()));
            }
        }
        status.insert(
            record.id.clone(),
            serde_json::json!({"configured": missing.is_empty(), "missing": missing}),
        );
    }
    Ok(serde_json::Value::Object(status))
}

fn main() {
    ClipboardWatcher::start();
    let index_db = IndexSQL::new();
    RecordSQL::new();
    let index_flag_presence = index_initialization_flags_present().unwrap_or((false, false));
    let mut config = config::Config::read_local_config().unwrap();
    if config.base.local_file_search_paths.is_none() {
        if let Err(error) =
            config::ensure_file_search_paths_initialized(default_file_search_paths())
        {
            eprintln!("初始化文件包含路径失败: {error}");
        }
        config = config::Config::read_local_config().unwrap();
    }
    let mut migrated_app_flag = None;
    let mut migrated_file_flag = None;
    if !index_flag_presence.0 {
        match index_db.has_app_indexes() {
            Ok(initialized) => {
                config.base.app_index_initialized = initialized;
                migrated_app_flag = Some(initialized);
            }
            Err(error) => {
                eprintln!("读取应用索引状态失败，本次启动跳过自动扫描: {error}");
                config.base.app_index_initialized = true;
            }
        }
    }
    if !index_flag_presence.1 {
        match index_db.has_file_indexes() {
            Ok(initialized) => {
                config.base.file_index_initialized = initialized;
                migrated_file_flag = Some(initialized);
            }
            Err(error) => {
                eprintln!("读取文件索引状态失败，本次启动跳过自动扫描: {error}");
                config.base.file_index_initialized = true;
            }
        }
    }
    if config
        .base
        .local_file_search_paths
        .as_ref()
        .is_some_and(Vec::is_empty)
        && !config.base.file_index_initialized
    {
        config.base.file_index_initialized = true;
        migrated_file_flag = Some(true);
    }
    if migrated_app_flag.is_some() || migrated_file_flag.is_some() {
        if let Err(error) = save_index_initialization_flags(migrated_app_flag, migrated_file_flag) {
            eprintln!("保存索引初始化状态失败: {error}");
        }
    }
    let hotkey_awaken = config.base.hotkey_awaken.clone();
    let hotkey_clipboard = config.base.hotkey_clipboard.clone();
    let hotkey_file_jump = config.base.hotkey_file_jump.clone();
    api::snippets::update_settings(
        config.base.snippets_enabled,
        config.base.snippet_trigger.clone(),
        config.base.text_snippets.clone(),
    );
    api::snippets::start();

    tauri::Builder::default()
        .plugin(win_file_drop::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let index_service = api::file_watcher::FileIndexUpdateService::start();
            app.manage(Mutex::new(index_service));
            app.manage(Mutex::new(None::<api::file_watcher::FileWatcher>));
            restart_file_watcher(app.handle());
            #[cfg(target_os = "windows")]
            {
                let watcher = match api::windows_app_watcher::WindowsAppWatcher::start(
                    app.handle().clone(),
                ) {
                    Ok(watcher) => Some(watcher),
                    Err(error) => {
                        eprintln!(
                            "[AppIndexWatcher] AppsFolder 监听启动失败，应用继续运行: {error}"
                        );
                        None
                    }
                };
                app.manage(Mutex::new(watcher));
            }
            app.manage(api::listary_jump::ListaryJumpHandle::start());
            let bootstrap_app = app.handle().clone();
            let bootstrap_config = config.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if !bootstrap_config.base.app_index_initialized {
                    match create_app_index_to_sql(bootstrap_app.clone()) {
                        Ok(()) => {}
                        Err(error) => eprintln!("首次应用索引扫描失败: {error}"),
                    }
                }

                if !bootstrap_config.base.file_index_initialized {
                    let paths = bootstrap_config
                        .base
                        .local_file_search_paths
                        .as_ref()
                        .cloned()
                        .unwrap_or_default();
                    if !paths.is_empty() && !FILE_INDEX_RUNNING.swap(true, Ordering::AcqRel) {
                        let service = bootstrap_app
                            .state::<Mutex<api::file_watcher::FileIndexUpdateService>>();
                        let _ = service.lock().unwrap().begin_rebuild();
                        let result = create_file_index_to_sql(bootstrap_app.clone());
                        let _ = service.lock().unwrap().finish_rebuild();
                        match result {
                            Ok(()) => {}
                            Err(error) => eprintln!("首次文件索引扫描失败: {error}"),
                        }
                        FILE_INDEX_RUNNING.store(false, Ordering::Release);
                    }
                }
            });
            let plugins_dir = crate::utils::dirs::app_plugins_dir()?;
            app.asset_protocol_scope()
                .allow_directory(plugins_dir, true)?;
            shortcut(app, &hotkey_awaken, &hotkey_clipboard, &hotkey_file_jump);
            utils::window::disable_system_menu(app)?;
            utils::window::set_window_shadow(app);
            println!("{:?}", &hotkey_awaken);
            let main_window = app.get_window("skylark").unwrap();
            let position = main_window.outer_position().unwrap();
            println!("{:?}", position);

            let open_item = MenuItem::with_id(app, "tray-open", "打开应用", true, None::<&str>)?;
            let clipboard_item =
                MenuItem::with_id(app, "tray-clipboard", "剪贴板", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "tray-settings", "设置", true, None::<&str>)?;
            let components_item =
                MenuItem::with_id(app, "tray-components", "组件库", true, None::<&str>)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>)?;
            let tray_menu = Menu::with_items(
                app,
                &[
                    &open_item,
                    &clipboard_item,
                    &settings_item,
                    &components_item,
                    &separator,
                    &quit_item,
                ],
            )?;
            if let Some(tray) = app.tray_by_id("main") {
                tray.set_menu(Some(tray_menu))?;
            }
            Ok(())
        })
        .on_tray_icon_event(|app, event| {
            if let TrayIconEvent::DoubleClick { id, button, .. } = event {
                if id.as_ref() == "main" && button == MouseButton::Left {
                    let _ = app.emit("window-show-request", ());
                }
            }
        })
        .on_menu_event(|app, event| {
            let event_name = match event.id().as_ref() {
                "tray-open" => Some("window-show-request"),
                "tray-clipboard" => Some("clipboard-show-request"),
                "tray-settings" => Some("settings-show-request"),
                "tray-components" => Some("components-show-request"),
                "quit" => {
                    app.exit(0);
                    None
                }
                _ => None,
            };
            if let Some(event_name) = event_name {
                let _ = app.emit(event_name, ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            search_keyword,
            create_file_index,
            create_app_index,
            rebuild_index,
            get_file_index_status,
            open_app,
            open_url,
            get_file_icon,
            run_python_script,
            run_python_plugin,
            probe_python_interpreter,
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
            get_auto_launch_enabled,
            set_auto_launch_enabled,
            get_index_settings,
            save_index_settings,
            get_snippet_settings,
            save_snippet_settings,
            get_plugin_settings,
            save_plugin_settings,
            clear_plugin_settings,
            get_plugin_settings_status,
            add_custom_app_index,
            get_custom_app_indexes,
            delete_custom_app_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_file_watch_config() -> api::file_watcher::WatchConfig {
    let config = config::Config::read_local_config().unwrap_or_default();
    build_file_watch_config_from(&config.base)
}

fn build_file_watch_config_from(config: &config::BaseConfig) -> api::file_watcher::WatchConfig {
    let roots = config
        .local_file_search_paths
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect();
    api::file_watcher::WatchConfig {
        roots,
        excluded_paths: config
            .local_file_search_exclude_paths
            .clone()
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        excluded_extensions: config.local_file_search_exclude_types.clone(),
        ..Default::default()
    }
}

fn restart_file_watcher(app: &AppHandle) {
    if let Some(state) = app.try_state::<Mutex<Option<api::file_watcher::FileWatcher>>>() {
        let mut guard = state.lock().unwrap();
        if let Some(watcher) = guard.as_mut() {
            watcher.stop_in_place();
        }
        *guard = None;
        let config = build_file_watch_config();
        if config.roots.is_empty() {
            println!("[FileWatcher] 文件包含路径为空，未启动监听");
            return;
        }
        let sender = app
            .state::<Mutex<api::file_watcher::FileIndexUpdateService>>()
            .lock()
            .unwrap()
            .sender();
        println!("[FileWatcher] 监听配置目录: {:?}", config.roots);
        match api::file_watcher::FileWatcher::start(config, move |events| {
            let changes = api::file_watcher::events_to_index_changes(
                &events,
                api::explorer::file_path_to_index,
            );
            println!(
                "[FileWatcher] 收到 {} 个事件，转换为 {} 个索引变更",
                events.len(),
                changes.len()
            );
            if !changes.is_empty() {
                if let Err(error) = sender.send(api::file_watcher::IndexMessage::Batch(changes)) {
                    eprintln!("[FileWatcher] 无法提交增量索引任务: {error}");
                }
            }
        }) {
            Ok(watcher) => {
                *guard = Some(watcher);
                println!("[FileWatcher] 文件监听已启动");
            }
            Err(error) => eprintln!("[FileWatcher] 文件监听启动失败，应用继续运行: {error}"),
        }
    }
}

fn stop_file_watcher(app: &AppHandle) {
    if let Some(state) = app.try_state::<Mutex<Option<api::file_watcher::FileWatcher>>>() {
        let mut guard = state.lock().unwrap();
        if let Some(watcher) = guard.as_mut() {
            watcher.stop_in_place();
        }
        *guard = None;
    }
}

// 启动前会再次确认旧句柄已清理，供配置变更的串行流程调用。
fn start_file_watcher(app: &AppHandle) {
    restart_file_watcher(app);
}

fn default_file_search_paths() -> Vec<String> {
    let mut paths = Vec::<PathBuf>::new();

    if let Some(user_profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        for directory in [
            "Desktop",
            "Documents",
            "Downloads",
            "Pictures",
            "Music",
            "Videos",
        ] {
            let path = user_profile.join(directory);
            if path.is_dir() {
                paths.push(path);
            }
        }
    }
    if let Some(one_drive) = std::env::var_os("OneDrive").map(PathBuf::from) {
        if one_drive.is_dir() {
            paths.push(one_drive);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let system_drive = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".to_string())
            .trim_end_matches('\\')
            .to_ascii_lowercase();
        paths.extend(
            api::explorer::get_drives()
                .into_iter()
                .filter(|(_, drive_type)| drive_type == "Fixed Drive")
                .map(|(path, _)| PathBuf::from(path))
                .filter(|path| {
                    path.to_string_lossy()
                        .trim_end_matches('\\')
                        .to_ascii_lowercase()
                        != system_drive
                }),
        );
    }

    paths.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    paths.dedup_by(|left, right| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    });
    paths
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}
