// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::collections::HashMap;
mod api;
mod utils;
mod config;

use tauri::{AppHandle, CustomMenuItem, GlobalShortcutManager, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu, Window, WindowEvent};
use crate::api::clipboard::ClipboardWatcher;
use rayon::prelude::*;
use walkdir::DirEntry;
use std::path::Path;
use std::thread;
use libc::stat;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::api::explorer::{create_app_index_to_sql, create_file_index_to_sql};
use crate::utils::database::{RecordSQL, IndexSQL, FileIndex};

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
fn search_keyword(component_name: &str, input_value: &str, offset: i32, params: HashMap<String, String>) -> Vec<SearchResult>
// where
//     T: From<HashMap<String, String>> + From<FileIndex>,
{
    println!("执行搜索 {:?} 关键词 {:?} 参数 {:?}", component_name, input_value, params);
    let comps: Vec<HashMap<String, String>> = Vec::new();
    if component_name == "" {
        return api::explorer::search_app_index(input_value, offset)
            .into_iter().map(SearchResult::File).collect();
    } else if component_name == "文件搜索" {
        let result = api::explorer::search_file_index(input_value, offset);
        println!("文件搜索结果 {:?}", result.len());
        let result = result.into_iter().map(SearchResult::File).collect();
        return result;
    }
    return comps.into_iter().map(SearchResult::Map).collect();
}


#[cfg(target_os = "macos")]
pub fn set_window_show(main_window: &Window) {
    // let main_window = state.app_handle.get_window("skylark").unwrap();
    main_window
        .emit("window-focus", true)
        .expect("Failed to emit event");
}

#[cfg(target_os = "windows")]
pub fn set_window_show(main_window: &Window) {
    // let main_window = state.app_handle.get_window("skylark").unwrap();
    main_window.emit("window-focus", true).expect("Failed to emit event");
}

#[tauri::command]
fn create_file_index(state: State<'_, AppState>) {
    let app_handle = state.app_handle.clone();
    create_file_index_to_sql(app_handle);
}

#[tauri::command]
fn create_app_index(state: State<'_, AppState>) {
    let app_handle = state.app_handle.clone();
    create_app_index_to_sql(app_handle);
}

#[tauri::command]
fn rebuild_index(state: State<'_, AppState>) {
    println!("rebuild index");
    let _ = IndexSQL::new().clear_data("file");
    create_file_index(state);
}

fn main() {
    // ClipboardWatcher::start();
    // IndexSQL::new();
    // RecordSQL::new();
    let config = config::Config::read_local_config().unwrap();
    let config_ = config.clone();

    tauri::Builder::default()
        .system_tray(
            SystemTray::new().with_menu(
                SystemTrayMenu::new()
                    .add_item(CustomMenuItem::new("show", "Show").accelerator(config_.base.hotkey_awaken.clone())),
            ),
        )
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                if id.as_str() == "show" {
                    utils::window::set_window_show();
                }
            }
            _ => {}
        })
        .setup(move |app| {
            app.manage(AppState {
                app_handle: app.handle(),
            });

            utils::window::set_window_shadow(app);

            let mut shortcut_manager = app.global_shortcut_manager();
            let main_window = app.get_window("skylark").unwrap();
            let position = main_window.outer_position().unwrap();
            println!("{:?}",position);
            // 注册快捷键
            shortcut_manager.register(&config.base.hotkey_awaken, move || {
                let main_window_clone = main_window.clone();
                set_window_show(&main_window_clone);
            })
                .expect("Failed to register global shortcut");


            let main_window = app.get_window("skylark").unwrap();

            main_window.on_window_event({
                let main_window = main_window.clone();
                move |event| match event {
                    WindowEvent::Focused(focused) => {
                        if !focused {
                            println!("Window lost focus.");
                            main_window
                                .emit("window-focus", focused)
                                .expect("Failed to emit event");
                        }
                    }
                    &_ => {}
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_keyword,
            create_file_index,
            create_app_index,
            rebuild_index,
            api::shell::open_app,
            api::shell::open_url,
            api::shell::get_file_icon,
            api::shell::run_python_script,
            api::shell::clipboard_control,
            api::shell::write_txt,
            api::shell::read_txt,
            api::shell::append_txt,
            api::shell::open_file,
            api::explorer::read_app_info,
            api::explorer::open_explorer,
            api::explorer::read_file_to_base64,
            api::explorer::read_icns_to_base64,
            utils::window::set_window_show,
            api::clipboard::get_history_all,
            api::clipboard::get_history_id,
            api::clipboard::get_history_part,
            api::clipboard::get_history_search,
            config::plugins::load_plugins,
            utils::dirs::get_app_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


#[test]
#[allow(unused)]
fn test_walkdir() {
    use walkdir::WalkDir;
    use chrono::Duration;

    fn is_hidden(entry: &DirEntry) -> bool {
        entry.file_name().to_str().map_or(false, |s| s.starts_with('.'))
    }
    tauri::async_runtime::spawn(async {
        WalkDir::new("/Users/starsxu/Music/Music/Media.localized")
            .into_iter()
            .filter_entry(|entry| !is_hidden(entry)) // 在遍历之前先过滤隐藏的文件夹
            .filter_map(Result::ok)
            .for_each(|entry| {
                println!("{:?}", entry.path());
                println!("外 {:?}", entry.file_name());
            });
    });
    println!("hello");
    thread::sleep(Duration::seconds(3).to_std().unwrap());
}