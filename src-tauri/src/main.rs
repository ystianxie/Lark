// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::collections::HashMap;
mod api;
mod utils;
mod config;
use tauri::{
    AppHandle, CustomMenuItem, GlobalShortcutManager, Manager, SystemTray, SystemTrayEvent,
    SystemTrayMenu, Window, WindowEvent,
};

#[derive(Clone)]
struct AppState {
    app_handle: AppHandle,
}


#[tauri::command(rename_all = "camelCase")]
fn search_keyword(component_name: &str, input_value: &str) -> Vec<HashMap<String, String>> {
    println!("执行搜索 {:?} 关键词 {:?}", component_name, input_value);
    let comps: Vec<HashMap<String, String>> = Vec::new();
    if component_name == "" {
        return search_all_app(input_value);
    } else if component_name == "搜索文件" {
        return api::explorer::search_files(input_value);
    }
    return comps;
}

#[tauri::command(rename_all = "camelCase")]
fn search_all_app(input_value: &str) -> Vec<HashMap<String, String>> {
    let applications = api::explorer::get_all_app(input_value);
    return applications;
}

#[cfg(target_os = "macos")]
pub fn set_window_show(main_window: &Window) {
    // let main_window = state.app_handle.get_window("skylark").unwrap();
    main_window
        .emit("window-focus", true)
        .expect("Failed to emit event");
}


fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState {
                app_handle: app.handle(),
            });

            utils::window::set_window_shadow(app);

            let mut shortcut_manager = app.global_shortcut_manager();
            let main_window = app.get_window("skylark").unwrap();

            // Register global shortcut
            shortcut_manager
                .register("Option+Space", move || {
                    // utils::set_window_show_macos();
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
        .system_tray(
            SystemTray::new().with_menu(
                SystemTrayMenu::new()
                    .add_item(CustomMenuItem::new("show", "Show").accelerator("Option+Space")),
            ),
        )
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                if id.as_str() == "show" {
                    utils::window::set_window_show_macos();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            search_keyword,
            search_all_app,
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
            utils::window::set_window_hide_macos,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
