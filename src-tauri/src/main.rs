// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
mod api;
mod config;
mod utils;

use crate::api::clipboard::{
    get_history_all, get_history_id, get_history_part, get_history_search, ClipboardWatcher,
};
use crate::api::explorer::{
    create_app_index_to_sql, create_file_index_to_sql, open_explorer, read_app_info,
    read_file_to_base64, read_icns_to_base64,
};
use crate::api::shell::{
    append_txt, clipboard_control, get_file_icon, open_app, open_file, open_url, read_txt,
    run_python_plugin, run_python_script, write_txt,
};
use crate::config::plugins::load_plugins;
use crate::utils::database::{FileIndex, IndexSQL, RecordSQL};
use crate::utils::dirs::get_app_dir;
use crate::utils::window::set_window_show;
use serde::{Deserialize, Serialize};
use tauri::{App, AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

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

fn shortcut(app: &mut App, hotkey: &str) {
    let window = app.get_webview_window("skylark").unwrap();
    app.handle()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut(hotkey)
                .unwrap_or(Default::default())
                .with_handler(move |_app, hotkey, event| {
                    if event.state == ShortcutState::Pressed {
                        if hotkey.matches(Modifiers::ALT, Code::Space) {
                            if window.is_visible().unwrap() {
                                window.hide().unwrap();
                                // window.emit("window-focus", false).unwrap();
                            } else {
                                // 先让前端重置非 panel 状态和窗口尺寸；前端准备完成后再显示。
                                window.emit("window-show-request", ()).unwrap();
                            }
                        }
                    }
                })
                .build(),
        )
        .unwrap();
}

fn main() {
    ClipboardWatcher::start();
    IndexSQL::new();
    RecordSQL::new();
    let config = config::Config::read_local_config().unwrap();
    let hotkey_awaken = config.base.hotkey_awaken.clone();

    tauri::Builder::default()
        .setup(move |app| {
            shortcut(app, &*hotkey_awaken);
            utils::window::set_window_shadow(app);
            println!("{:?}", &hotkey_awaken);
            let main_window = app.get_window("skylark").unwrap();
            let position = main_window.outer_position().unwrap();
            println!("{:?}", position);
            Ok(())
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
            get_app_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
