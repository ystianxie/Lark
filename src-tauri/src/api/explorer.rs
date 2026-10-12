use crate::config;
use crate::utils::database::{FileIndex, IndexSQL};
use crate::utils::icons;
use crate::utils::string_factory::text_to_pinyin;
use base64::{engine::general_purpose, Engine as _};
use icns::{IconFamily, IconType};
use pinyin::ToPinyin;
use plist::Value;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use walkdir::{DirEntry, WalkDir};
#[cfg(target_os = "windows")]
use windows::{
    core::PCWSTR,
    Win32::{
        System::Com::{
            CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
            COINIT_DISABLE_OLE1DDE,
        },
        UI::Shell::{ILCreateFromPathW, SHOpenFolderAndSelectItems},
    },
};

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::process::CommandExt;
use std::ptr;
use winapi::shared::minwindef::LPARAM;
use winapi::shared::ntdef::HANDLE;
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
};
use winapi::um::winuser::{
    EnumWindows, FindWindowW, GetWindowTextLengthW, GetWindowTextW, SetForegroundWindow,
    ShowWindow, SW_RESTORE,
};

pub fn to_pinyin(hans: &str) -> Vec<String> {
    let mut ret = Vec::new();
    for pinyin in hans.to_pinyin() {
        if let Some(pinyin) = pinyin {
            ret.push(pinyin.plain().to_string());
        }
    }

    return ret;
}

#[derive(Serialize, Deserialize, Default)]
pub struct AppInfo {
    pub name: String,
    pub icon: String,
    pub start: String,
}

pub fn search_files(keyword: &str) -> Vec<HashMap<String, String>> {
    let mut result = Vec::new();
    let search_path = "/Users/starsxu/";
    let fd_output = Command::new("fd")
        .args(&["-a", "-t", "f", keyword, "-p", search_path])
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");
    let head_output = Command::new("head")
        .args(&["-n", "10"]) // 限制结果数量为 10
        .stdin(fd_output.stdout.unwrap())
        .output()
        .expect("failed to execute head");
    // 检查命令是否成功执行
    if head_output.status.success() {
        let stdout = String::from_utf8_lossy(&head_output.stdout);
        println!("Command executed successfully {}", stdout);
        for row in stdout.split("\n") {
            let file_name = row
                .replace("\\", "/")
                .split("/")
                .last()
                .expect("REASON")
                .to_string();
            if file_name.contains(keyword) {
                let map: HashMap<String, String> = HashMap::from([
                    ("icon".to_string(), file_name.clone()),
                    ("title".to_string(), file_name.clone()),
                    ("desc".to_string(), row.to_string()),
                    ("data".to_string(), row.to_string()),
                    ("type".to_string(), "file".to_string()),
                ]);
                result.push(map);
                if result.len() > 9 {
                    break;
                }
            }
        }
    } else {
        eprintln!("Command failed");
    }

    result
}

pub fn search_file_index(keyword: &str, offset: i32) -> Vec<FileIndex> {
    let db = IndexSQL::new();
    match db.find_by_keyword("file", keyword, offset) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("file search failed: {error:#}");
            Vec::new()
        }
    }
}

/// 将文件系统路径转换为增量索引记录。路径不存在或无法读取元数据时返回 None。
pub fn file_path_to_index(path: &std::path::Path) -> Option<FileIndex> {
    let metadata = (0..4).find_map(|_| match std::fs::metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_millis(75));
            None
        }
    })?;
    let title = path.file_name()?.to_str()?.to_string();
    let (pinyin, abb) = text_to_pinyin(&title);
    let file_type = if metadata.is_dir() {
        "folder".to_string()
    } else {
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase()
    };
    let path = path.to_string_lossy().into_owned();
    Some(FileIndex {
        title,
        path,
        desc: String::new(),
        pinyin,
        abb,
        file_type,
        ..Default::default()
    })
}
pub fn search_app_index(keyword: &str, offset: i32) -> Vec<FileIndex> {
    let db = IndexSQL::new();
    if let Ok(result) = db.find_app(keyword, offset) {
        // Do not apply discovery filters here. This query also returns
        // explicitly added applications, which users are allowed to keep.
        return result;
    }
    vec![FileIndex {
        ..Default::default()
    }]
}

#[cfg(target_os = "windows")]
fn get_app_from_lnk(lnk_path: &str) -> Result<Vec<String>, String> {
    let shortcut = crate::api::windows_apps::read_shortcut_details(lnk_path)?;
    let advertised = crate::api::windows_apps::is_advertised_shortcut(lnk_path);
    let title = Path::new(lnk_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    if title.is_empty() {
        return Err("快捷方式标题为空".to_string());
    }
    let mut target = expand_app_environment(&shortcut.target);
    let recovered = if !is_launchable_shortcut_target(&target) {
        if let Some(candidate) = matching_working_directory_executable(
            &title,
            &expand_app_environment(&shortcut.working_directory),
        ) {
            target = candidate;
            true
        } else {
            false
        }
    } else {
        false
    };
    let known = shortcut.arguments.is_some();
    Ok(vec![
        title,
        target,
        shortcut.arguments.unwrap_or_default(),
        known.to_string(),
        advertised.to_string(),
        recovered.to_string(),
    ])
}

#[cfg(target_os = "windows")]
fn matching_working_directory_executable(title: &str, working_directory: &str) -> Option<String> {
    let title_key: String = title
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect();
    if title_key.is_empty() || !Path::new(working_directory).is_dir() {
        return None;
    }
    let mut matches = fs::read_dir(working_directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| {
                        stem.chars()
                            .filter(|character| character.is_ascii_alphanumeric())
                            .map(|character| character.to_ascii_lowercase())
                            .collect::<String>()
                            == title_key
                    })
                    .unwrap_or(false)
        });
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(first.to_string_lossy().into_owned())
}

pub fn get_apps(path: &str) -> Vec<HashMap<String, String>> {
    get_apps_with_depth(path, true, true, false, &[])
}

#[cfg(target_os = "windows")]
fn normalize_app_scan_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

#[cfg(not(target_os = "windows"))]
fn normalize_app_scan_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

fn is_app_scan_path_excluded(path: &str, excluded_paths: &[String]) -> bool {
    let path = normalize_app_scan_path(path);
    excluded_paths.iter().any(|excluded| {
        let excluded = normalize_app_scan_path(excluded);
        !excluded.is_empty()
            && (path == excluded
                || path
                    .strip_prefix(&excluded)
                    .map(|suffix| suffix.starts_with('\\') || suffix.starts_with('/'))
                    .unwrap_or(false))
    })
}

#[cfg(target_os = "windows")]
fn is_common_app_excluded_dir(entry: &fs::DirEntry) -> bool {
    const EXCLUDED: &[&str] = &[
        ".git",
        ".hg",
        ".svn",
        ".idea",
        ".vscode",
        "node_modules",
        "target",
        "build",
        "dist",
        "out",
        "vendor",
        "coverage",
        ".cache",
        ".turbo",
        ".next",
        ".nuxt",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        "venv",
        ".venv",
        "env",
        ".env",
        "runtime",
        "resources",
        "lib",
        "libs",
        "docs",
        "examples",
        "samples",
    ];
    entry
        .file_name()
        .to_str()
        .map(|name| {
            EXCLUDED
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn is_auxiliary_app_file(name: &str) -> bool {
    is_portable_auxiliary_app(name, name)
}

fn is_lnk_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".lnk")
}

/// Filter only applications whose decoded 32x32 RGBA pixels exactly match the
/// supplied `no_icon.txt` sample. Comparing pixels instead of PNG bytes avoids
/// differences in PNG compression and metadata. WndSpy.exe has a different
/// image and is kept.
#[cfg(target_os = "windows")]
fn is_filtered_app(path: &str, _title: &str) -> bool {
    const NO_ICON_BASE64: &str = include_str!("../../../no_icon.txt");
    let icon_result = std::panic::catch_unwind(|| icons::get_icon(path, 32));
    let Ok(Ok(bytes)) = icon_result else {
        return false;
    };
    let Ok(candidate) = image::load_from_memory(&bytes) else {
        return false;
    };
    let Ok(reference_bytes) = general_purpose::STANDARD.decode(NO_ICON_BASE64.trim()) else {
        return false;
    };
    let Ok(reference) = image::load_from_memory(&reference_bytes) else {
        return false;
    };
    let candidate = image::imageops::resize(
        &candidate.to_rgba8(),
        32,
        32,
        image::imageops::FilterType::Nearest,
    );
    let reference = image::imageops::resize(
        &reference.to_rgba8(),
        32,
        32,
        image::imageops::FilterType::Nearest,
    );
    let mut different = 0usize;
    let mut total_error = 0u64;
    for (left, right) in candidate.pixels().zip(reference.pixels()) {
        let error = left
            .0
            .iter()
            .zip(right.0.iter())
            .map(|(a, b)| i16::from(*a).abs_diff(i16::from(*b)) as u64)
            .sum::<u64>();
        total_error += error;
        if error > 48 {
            different += 1;
        }
    }
    // Small anti-aliasing/alpha differences are expected between shell icon
    // extraction paths. Require the structure and colors to match closely.
    different <= 96 && total_error <= 30_000
}

#[cfg(not(target_os = "windows"))]
fn is_filtered_app(_path: &str, _title: &str) -> bool {
    false
}

fn get_apps_with_depth(
    path: &str,
    recursive: bool,
    include_direct_files: bool,
    exclude_common_dirs: bool,
    excluded_paths: &[String],
) -> Vec<HashMap<String, String>> {
    if is_app_scan_path_excluded(path, excluded_paths) {
        return Vec::new();
    }
    println!("开始检索目录： {:?}", path.replace("\\", "/"));
    let mut applications = Vec::new();
    let entries = fs::read_dir(path);
    if entries.is_err() {
        return vec![];
    }
    for entry in entries.unwrap() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        #[cfg(target_os = "macos")]
        {
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => {
                    println!("类型获取失败{:?}", entry.path());
                    continue;
                }
            };
            if !file_type.is_dir() {
                continue;
            }
        }

        let file_name = entry.file_name();

        let app_name = match file_name.to_str() {
            Some(name) => name,
            None => continue,
        };
        let mut app_title: String = String::new();
        let mut app_path: String = match entry.path().to_str() {
            Some(path) => path.to_string(),
            None => continue,
        };
        let mut app_arguments = String::new();
        let mut arguments_known = true;
        let mut advertised_shortcut = false;
        #[cfg(target_os = "macos")]
        {
            if !app_name.ends_with(".app") {
                // println!("文件夹：{:?}", entry.path().to_str().unwrap());
                applications.extend(get_apps_with_depth(
                    entry.path().to_str().unwrap(),
                    true,
                    include_direct_files,
                    exclude_common_dirs,
                    excluded_paths,
                ));
                continue;
            }

            app_title = app_name.to_string().replace(".app", "");
            let current_dir = std::env::current_dir().expect("获取当前工作目录失败");
            println!("当前工作目录: {:?}", current_dir);

            let json_data = fs::read_to_string("src/api/system_app_name.json").expect("{}");
            let translations: HashMap<String, String> =
                serde_json::from_str(&json_data).expect("JSON 解析失败");

            if let Some(chinese_translation) = translations.get(&app_title) {
                println!("{} 的中文翻译是: {}", app_name, chinese_translation);
                app_title = chinese_translation.to_string();
            }

            let resources = &(entry.path().join("Contents/Resources/"));

            // 检查资源目录是否存在
            if !resources.exists() {
                continue;
            }
            let locales = ["zh-CN.lproj", "zh_CN.lproj", "zh-Hans.lproj", ""];
            for locale in &locales {
                let locale_path = resources.join(locale);

                if locale_path.exists() {
                    // 查找 InfoPlist.strings 文件
                    let info_plist_path = locale_path.join("InfoPlist.strings");
                    if info_plist_path.exists() {
                        // 读取 InfoPlist.strings 文件
                        let mut file = fs::File::open(&info_plist_path).expect("无法打开文件");
                        let mut bom = [0; 4];
                        file.read(&mut bom).expect("msg");
                        let encoding = match bom {
                            [0xFF, 0xFE, ..] => UTF_16LE,
                            [0xFE, 0xFF, ..] => UTF_16BE,
                            _ => UTF_8, // 默认为 UTF-8
                        };

                        // 将文件指针重置到开头
                        file = fs::File::open(&info_plist_path).expect("无法打开文件");
                        let transcoded = DecodeReaderBytesBuilder::new()
                            .encoding(Some(encoding))
                            .build(file);
                        let reader = BufReader::new(transcoded);

                        for line in reader.lines() {
                            let line: String = line.expect("读取配置行失败");
                            let re = Regex::new(r#""CFBundleDisplayName"\s*=\s*"(.*?)""#).unwrap();
                            let re2 = Regex::new(r#"CFBundleDisplayName\s*=\s*"(.*?)""#).unwrap();
                            if let Some(caps) = re.captures(&line) {
                                // println!("Apples: {}", caps.get(1).unwrap().as_str());
                                app_title = caps.get(1).unwrap().as_str().to_string();
                                break;
                            } else if let Some(caps) = re2.captures(&line) {
                                app_title = caps.get(1).unwrap().as_str().to_string();
                                break;
                            }
                        }
                    }
                }
            }
        }

        // TODO Windows下检查目录程序
        #[cfg(target_os = "windows")]
        {
            if entry.path().is_dir() {
                if !recursive {
                    continue;
                }
                if exclude_common_dirs && is_common_app_excluded_dir(&entry) {
                    continue;
                }
                println!(
                    "文件夹：{:?}",
                    entry.path().to_str().unwrap().replace("\\", "/")
                );
                if let Some(path) = entry.path().to_str() {
                    applications.extend(get_apps_with_depth(
                        path,
                        true,
                        include_direct_files,
                        exclude_common_dirs,
                        excluded_paths,
                    ));
                }
                continue;
            }
            let extension = entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(str::to_ascii_lowercase);
            if include_direct_files && !is_lnk_name(app_name) && is_auxiliary_app_file(app_name) {
                continue;
            }
            let is_lnk = extension.as_deref() == Some("lnk");
            if is_lnk && !app_name.starts_with("卸载") {
                let shortcut_path_buf = entry.path();
                let shortcut_path = match shortcut_path_buf.to_str() {
                    Some(path) => path,
                    None => continue,
                };
                if let Ok(app) = get_app_from_lnk(shortcut_path) {
                    let (Some(title), Some(path)) = (app.get(0), app.get(1)) else {
                        continue;
                    };
                    app_arguments = app.get(2).cloned().unwrap_or_default();
                    arguments_known = app.get(3).is_some_and(|value| value == "true");
                    advertised_shortcut = app.get(4).is_some_and(|value| value == "true");
                    let recovered_target = app.get(5).is_some_and(|value| value == "true");
                    app_title = title.clone();
                    app_path = path.clone();
                    println!(
                        "获取到应用:{app_title}--->{app_path}{}{}",
                        if advertised_shortcut {
                            "（Windows Installer 广告式快捷方式）"
                        } else {
                            ""
                        },
                        if recovered_target {
                            "（从工作目录恢复匹配 exe）"
                        } else {
                            ""
                        },
                    );
                    if is_auxiliary_shortcut(&app_title, &app_path) {
                        println!(
                            "[AppIndexTrace] 跳过开始菜单快捷方式（辅助程序过滤）: title={}, target={}",
                            app_title, app_path
                        );
                        continue;
                    }
                } else {
                    println!(
                        "[AppIndexTrace] 跳过开始菜单快捷方式（get_app_from_lnk 失败）: lnk={}",
                        shortcut_path
                    );
                    continue;
                }
                // continue;
            } else if include_direct_files && matches!(extension.as_deref(), Some("rdp" | "url")) {
                app_title = entry
                    .path()
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or(app_name)
                    .to_string();
            } else if include_direct_files && extension.as_deref() == Some("exe") {
                app_title = entry
                    .path()
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or(app_name)
                    .to_string();
            } else {
                continue;
            }
        }
        println!(
            "应用：{}，路径：{}",
            app_title,
            entry.path().to_str().unwrap()
        );
        let map: HashMap<String, String> = HashMap::from([
            (
                "icon".to_string(),
                if app_path.is_empty() {
                    entry.path().to_string_lossy().into_owned()
                } else {
                    app_path.clone()
                },
            ),
            ("title".to_string(), app_title.clone()),
            ("desc".to_string(), {
                #[cfg(target_os = "windows")]
                {
                    app_display_desc(&app_path, true)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    app_path.clone()
                }
            }),
            ("target_path".to_string(), app_path.clone()),
            (
                "data".to_string(),
                if is_lnk_name(app_name) {
                    entry.path().to_string_lossy().into_owned()
                } else {
                    app_path.clone()
                },
            ),
            ("arguments".to_string(), app_arguments),
            ("arguments_known".to_string(), arguments_known.to_string()),
            (
                "advertised_shortcut".to_string(),
                advertised_shortcut.to_string(),
            ),
            (
                "shortcut_path".to_string(),
                entry.path().to_string_lossy().into_owned(),
            ),
            ("type".to_string(), "app".to_string()),
        ]);
        applications.push(map);
    }
    applications
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_file_to_base64(path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("无法打开文件：{error}"))?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .map_err(|error| format!("无法读取文件内容：{error}"))?;
    Ok(general_purpose::STANDARD.encode(&contents))
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_icns_to_base64(path: &str) -> Result<String, String> {
    let file = File::open(path);
    if let Err(_) = file {
        return Ok("".to_string());
    }
    let icon_family = IconFamily::read(file.unwrap()).map_err(|e| e.to_string());
    let icon_family = match icon_family {
        Ok(icon_family) => icon_family,
        Err(e) => {
            return if e.contains("not an icns file") {
                read_file_to_base64(path)
            } else {
                Ok("".to_string())
            }
        }
    };
    let img_index = match path.contains("AirPort Utility.app") {
        true => 2,
        false => 0,
    };
    let image_type = icon_family.available_icons()[img_index];
    if let Some(img_buffer) = icon_family
        .elements
        .iter()
        .find(|e| e.ostype == image_type.ostype())
    {
        if format!("{:?}", image_type).contains("RGBA") {
            let base64_image = general_purpose::STANDARD.encode(&img_buffer.data);
            return Ok(base64_image);
        }
    }
    let icon = icon_family
        .get_icon_with_type(IconType::RGB24_128x128)
        .or_else(|_| icon_family.get_icon_with_type(IconType::RGBA32_128x128))
        .or_else(|_| icon_family.get_icon_with_type(IconType::RGBA32_128x128_2x))
        .or_else(|_| icon_family.get_icon_with_type(IconType::RGBA32_256x256))
        .or_else(|_| icon_family.get_icon_with_type(IconType::RGBA32_256x256_2x))
        .or_else(|_| icon_family.get_icon_with_type(IconType::RGBA32_64x64))
        .or_else(|_| icon_family.get_icon_with_type(IconType::RGB24_48x48))
        .or_else(|_| icon_family.get_icon_with_type(image_type))
        .map_err(|_| "No suitable icon found".to_string())?;
    let mut buffer = Cursor::new(Vec::new());
    icon.write_png(&mut buffer).map_err(|e| e.to_string())?;
    let base64_image = general_purpose::STANDARD.encode(buffer.get_ref());
    Ok(base64_image)
}

// 获取文件图标
#[tauri::command]
fn read_icon_to_base64(path: String) -> String {
    #[cfg(target_os = "windows")]
    if path.to_ascii_lowercase().starts_with("shell:") || path.starts_with("::{") {
        // Shell parsing names are not filesystem paths. Never pass them to the
        // generic file icon fallback, which can return a misleading blank icon.
        return crate::api::windows_apps::read_shell_icon_base64(&path).unwrap_or_default();
    }
    // Use the shell/DIB conversion first. It preserves old 16/256-colour
    // resources such as WndSpy.exe that the hand-built ICO conversion can
    // decode incorrectly.
    #[cfg(target_os = "windows")]
    if let Ok(Ok(icon)) = std::panic::catch_unwind(|| windows_icons::get_icon_base64_by_path(&path))
    {
        if !icon.is_empty() {
            return icon;
        }
    }
    // Keep the original converter as a fallback for paths where the shell
    // cannot provide a usable HICON.
    if let Ok(Ok(buffer)) = std::panic::catch_unwind(|| icons::get_icon(&path, 128)) {
        return general_purpose::STANDARD.encode(buffer);
    }
    String::from("")
}
#[tauri::command(rename_all = "camelCase")]
pub fn read_app_info(app_path: &str) -> String {
    println!("app_path: {}", app_path);
    let plist_path = Path::new(app_path).join("Contents/Info.plist");
    // let mut file = File::open(&plist_path).expect("无法打开文件");
    if let Ok(mut file) = File::open(&plist_path) {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("无法读取文件内容");
        let plist_value: Value = plist::from_bytes(&buffer).expect("无法解析plist文件");
        // 打印所有键值对
        let mut icon_name = String::new();
        if let Value::Dictionary(dict) = plist_value {
            for (key, value) in dict {
                if key == "CFBundleIconFile" || key == "CFBundleIcons" {
                    if let Some(icon_str) = value.as_string() {
                        icon_name = icon_str.to_string();
                    }
                }
            }
        } else {
            println!("Info.plist is not a dictionary");
        }
        let resources = PathBuf::from(&app_path).join("Contents/Resources");

        if icon_name.is_empty() {
            icon_name = "AppIcon".to_string();
            let path = resources.join(format!("{icon_name}.icns"));
            if !path.exists() {
                for entry in resources.read_dir().unwrap() {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_) => continue,
                    };
                    if entry.path().extension().unwrap_or("".as_ref()) == "icns" {
                        icon_name = entry.file_name().to_string_lossy().into_owned();
                        break;
                    }
                }
                return "文件不存在！".to_string();
            }
        }
        if !icon_name.ends_with(".icns") {
            icon_name.push_str(".icns");
        }
        let icon_path = resources.join(icon_name);
        println!("获取文件图标：{:?}", icon_path);
        icon_path.to_string_lossy().into_owned()
    } else {
        "文件不存在！".to_string()
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_explorer(path: &str) -> String {
    match open_explorer_result(path) {
        Ok(()) => "打开成功！".to_string(),
        Err(error) => {
            eprintln!("打开资源管理器失败：{error}");
            "打开失败！".to_string()
        }
    }
}

pub fn open_explorer_result(path: &str) -> Result<(), String> {
    let target = Path::new(path);
    if !target.exists() {
        return Err(format!("目标路径不存在：{path}"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Err(native_error) = open_explorer_with_shell(path) {
            // The native Shell API is the reliable path-aware implementation. If
            // COM/Shell is unavailable, fall back to the containing directory,
            // never to the file itself (which could launch the associated app).
            let containing_directory = if target.is_dir() {
                target
            } else {
                target.parent().unwrap_or(target)
            };
            Command::new("explorer")
                .arg(containing_directory)
                .spawn()
                .map_err(|fallback_error| {
                    format!(
                        "资源管理器无法打开目标路径：{native_error}；回退打开目录也失败：{fallback_error}"
                    )
                })?;
        }

        if let Some(folder_name) = target
            .parent()
            .unwrap_or(target)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(150));
                activate_explorer_window(&folder_name);
            });
        }

        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let is_directory = target.is_dir();
        let containing_directory = if is_directory {
            target
        } else {
            target.parent().unwrap_or(target)
        };

        if cfg!(target_os = "macos") {
            Command::new("open")
                .arg(containing_directory)
                .spawn()
                .map_err(|error| format!("无法打开目录：{error}"))?;
        } else if cfg!(target_os = "linux") {
            Command::new("xdg-open")
                .arg(containing_directory)
                .spawn()
                .map_err(|error| format!("无法打开目录：{error}"))?;
        } else {
            return Err("当前系统不支持打开资源管理器".into());
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn open_explorer_with_shell(path: &str) -> Result<(), String> {
    let mut wide_path: Vec<u16> = path.encode_utf16().collect();
    wide_path.push(0);

    unsafe {
        let com_initialized =
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
        let pidl = ILCreateFromPathW(PCWSTR::from_raw(wide_path.as_mut_ptr()));

        if pidl.is_null() {
            if com_initialized {
                CoUninitialize();
            }
            return Err(format!("无法解析目标路径：{path}"));
        }

        let result = SHOpenFolderAndSelectItems(pidl, None, 0);
        CoTaskMemFree(Some(pidl as *const std::ffi::c_void));
        if com_initialized {
            CoUninitialize();
        }

        result.map_err(|error| format!("资源管理器无法打开目标路径：{error}"))
    }
}

#[cfg(target_os = "windows")]
fn activate_explorer_window(folder_name: &str) {
    let wide_title: Vec<u16> = OsStr::new(folder_name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let hwnd = unsafe { FindWindowW(ptr::null_mut(), wide_title.as_ptr()) };
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    } else {
        find_windows_with_partial_title(folder_name);
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn get_drives() -> Vec<(String, String)> {
    let mut drives = Vec::new();
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;
    const DRIVE_RAMDISK: u32 = 6;
    let drive_bits = unsafe { GetLogicalDrives() };

    for i in 0..26 {
        if (drive_bits & (1 << i)) != 0 {
            let drive_letter = format!("{}:\\", (b'A' + i) as char);
            let drive_letter_w: Vec<u16> = OsString::from(&drive_letter)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let drive_type = unsafe { GetDriveTypeW(drive_letter_w.as_ptr()) };

            let drive_type_str = match drive_type {
                DRIVE_FIXED => "Fixed Drive",
                DRIVE_REMOVABLE => "Removable Drive",
                DRIVE_CDROM => "CD-ROM Drive",
                DRIVE_REMOTE => "Network Drive",
                DRIVE_RAMDISK => "RAM Disk",
                _ => "Unknown",
            };

            drives.push((drive_letter, drive_type_str.to_string()));
        }
    }

    drives
}

fn file_scanning(
    app_handle: AppHandle,
    roots: Vec<String>,
    skip_dirs: Vec<String>,
    skip_extensions: Vec<String>,
) -> Result<(), String> {
    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map_or(false, |s| s.starts_with('.'))
    }

    fn should_skip_file(entry: &DirEntry, skip_extensions: &[String]) -> bool {
        entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                skip_extensions.iter().any(|value| {
                    value
                        .trim()
                        .trim_start_matches('.')
                        .eq_ignore_ascii_case(ext)
                })
            })
    }

    fn should_skip_dir(entry: &DirEntry, skip_dirs: &[String]) -> bool {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with('$') || name.starts_with('.'))
        {
            return true;
        }
        skip_dirs
            .iter()
            .any(|dir| super::file_watcher::path_is_under(entry.path(), Path::new(dir)))
    }
    println!("开始扫描固定磁盘:{:?}", roots);
    let skip_extensions_data = Arc::new(skip_extensions);
    let skip_dirs_data = Arc::new(skip_dirs);
    let files = Arc::new(Mutex::new(Vec::new()));
    {
        let main_window = app_handle.get_window("skylark");
        let mut index_db = IndexSQL::new();
        let generation = match index_db.begin_file_generation() {
            Ok(generation) => generation,
            Err(error) => return Err(format!("无法开始文件索引代次: {error}")),
        };
        for root in roots {
            let walker = WalkDir::new(root).follow_links(false).into_iter();
            for entry in walker.filter_entry(|entry| {
                !is_hidden(entry)
                    && !should_skip_file(entry, &skip_extensions_data)
                    && !should_skip_dir(entry, &skip_dirs_data)
            }) {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        eprintln!("跳过目录项: {error}");
                        continue;
                    }
                };
                let title = match entry.file_name().to_str() {
                    Some(value) => value.to_string(),
                    None => continue,
                };
                let file_path = entry.path().to_string_lossy().into_owned();
                let file_type = if entry.file_type().is_dir() {
                    "folder".to_string()
                } else {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                };
                let (pinyin, abb) = text_to_pinyin(&title);
                let mut batch = files.lock().unwrap();
                batch.push(FileIndex {
                    title,
                    path: file_path,
                    pinyin,
                    abb,
                    file_type,
                    ..Default::default()
                });
                if batch.len() >= 500 {
                    let current = std::mem::take(&mut *batch);
                    drop(batch);
                    if let Err(error) = index_db.insert_file_generation(generation, &current) {
                        return Err(format!("写入文件索引失败: {error}"));
                    }
                    if let Some(window) = &main_window {
                        let _ = window.emit("file_index_count", current.len());
                    }
                }
            }
        }
        let remaining = std::mem::take(&mut *files.lock().unwrap());
        if let Err(error) = index_db
            .insert_file_generation(generation, &remaining)
            .and_then(|_| index_db.commit_file_generation(generation))
        {
            return Err(format!("提交文件索引失败: {error}"));
        } else if let Some(window) = &main_window {
            let _ = window.emit("file_index_complete", ());
        }
    }
    println!("磁盘扫描完成！");
    Ok(())
}

#[cfg(target_os = "windows")]
type AppMetadata = HashMap<String, String>;

#[cfg(target_os = "windows")]
fn collect_secondary_apps(app_handle: &AppHandle) -> (Vec<AppMetadata>, Vec<AppMetadata>) {
    let mut start_menu_roots = Vec::new();
    if let Some(path) = std::env::var_os("APPDATA") {
        start_menu_roots.push(PathBuf::from(path).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(path) = std::env::var_os("PROGRAMDATA") {
        start_menu_roots.push(PathBuf::from(path).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    let mut desktop_roots = Vec::new();
    if let Ok(path) = app_handle.path().desktop_dir() {
        desktop_roots.push(path);
    }
    if let Some(path) = std::env::var_os("PUBLIC") {
        desktop_roots.push(PathBuf::from(path).join("Desktop"));
    }
    let scan = |mut roots: Vec<PathBuf>, recursive: bool| {
        roots.sort_by_key(|path| normalize_app_launch_key(&path.to_string_lossy()));
        roots.dedup_by(|a, b| {
            normalize_app_launch_key(&a.to_string_lossy())
                == normalize_app_launch_key(&b.to_string_lossy())
        });
        roots
            .iter()
            .filter(|path| path.is_dir())
            .flat_map(|path| {
                get_apps_with_depth(&path.to_string_lossy(), recursive, true, false, &[])
            })
            .collect()
    };
    (scan(start_menu_roots, true), scan(desktop_roots, false))
}

#[cfg(target_os = "windows")]
fn associated_shortcut_arguments(
    title: &str,
    target: &str,
    start_menu: &[AppMetadata],
    desktop: &[AppMetadata],
) -> Option<String> {
    if target.trim().is_empty() {
        return None;
    }
    let target = normalize_app_launch_key(target);
    let mut found: Option<&str> = None;
    for candidate in start_menu.iter().chain(desktop) {
        // A namespace GUID identifies the same target across localized names
        // (e.g. Run / 运行). EXE associations still require matching titles.
        if !candidate.get("data").is_some_and(|path| is_lnk_name(path))
            || !(target.starts_with("::{")
                || candidate
                    .get("title")
                    .is_some_and(|name| name.trim().to_lowercase() == title.trim().to_lowercase()))
            || !candidate.get("target_path").is_some_and(|path| {
                !path.trim().is_empty() && normalize_app_launch_key(path) == target
            })
        {
            continue;
        }
        // Any unavailable/conflicting parameter set makes the association ambiguous.
        if !candidate
            .get("arguments_known")
            .is_some_and(|known| known == "true")
        {
            return None;
        }
        let arguments = candidate.get("arguments")?.as_str();
        if found.is_some_and(|previous| previous != arguments) {
            return None;
        }
        found = Some(arguments);
    }
    found.map(str::to_string)
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct AppPathChecks {
    missing: HashMap<String, bool>,
    remote_drives: HashMap<u8, bool>,
    cache_hits: usize,
    skipped: usize,
    elapsed: Duration,
}

#[cfg(target_os = "windows")]
fn local_app_file_path(value: &str) -> Option<String> {
    let value = expand_app_environment(value);
    let value = value.trim().trim_matches('"').replace('/', "\\");
    // Some Win32 AppsFolder IDs contain the executable path itself.
    let path = if value.to_ascii_lowercase().starts_with("shell:appsfolder\\") {
        &value["shell:appsfolder\\".len()..]
    } else {
        value.as_str()
    };
    let path = path.strip_prefix(r"\\?\").unwrap_or(path);
    let bytes = path.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || path.contains(['%', '\0', '�'])
    {
        return None;
    }
    Some(path.to_string())
}

#[cfg(target_os = "windows")]
fn app_display_desc(target_path: &str, show_path: bool) -> String {
    if !show_path
        || target_path
            .trim()
            .to_ascii_lowercase()
            .starts_with("shell:")
    {
        return String::new();
    }
    local_app_file_path(target_path)
        .filter(|path| !path.to_ascii_lowercase().ends_with(".ico"))
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
impl AppPathChecks {
    fn missing_file(&mut self, value: &str) -> Option<String> {
        let path = local_app_file_path(value)?;
        let key = path.to_lowercase();
        if let Some(&missing) = self.missing.get(&key) {
            self.cache_hits += 1;
            return missing.then_some(path);
        }
        let started = std::time::Instant::now();
        let drive = path.as_bytes()[0].to_ascii_uppercase();
        let remote = *self.remote_drives.entry(drive).or_insert_with(|| {
            let root = [drive as u16, b':' as u16, b'\\' as u16, 0];
            unsafe {
                windows::Win32::Storage::FileSystem::GetDriveTypeW(PCWSTR(root.as_ptr()))
                    == windows::Win32::System::WindowsProgramming::DRIVE_REMOTE
            }
        });
        // Avoid blocking discovery on network shares or mapped network drives.
        let missing = if remote {
            false
        } else {
            match Path::new(&path).try_exists() {
                Ok(exists) => !exists,
                Err(error) => {
                    println!(
                        "[AppIndexTrace] 保留入口（文件存在检查失败）: file={}, error={}",
                        path, error
                    );
                    false
                }
            }
        };
        self.elapsed += started.elapsed();
        self.missing.insert(key, missing);
        missing.then_some(path)
    }

    fn should_skip(
        &mut self,
        title: &str,
        path: &str,
        target_path: &str,
        advertised_shortcut: bool,
        source: &str,
    ) -> bool {
        let missing = self
            .missing_file(path)
            .map(|file| ("path", file))
            .or_else(|| {
                // An MSI advertised link's target is not necessarily its executable.
                if advertised_shortcut {
                    None
                } else {
                    self.missing_file(target_path)
                        .map(|file| ("target_path", file))
                }
            });
        if let Some((field, file)) = missing {
            self.skipped += 1;
            println!(
                "[AppIndexTrace] 跳过失效入口（文件不存在）: source={}, title={}, path={}, target_path={}, missing_field={}, missing_file={}",
                source, title, path, target_path, field, file
            );
            return true;
        }
        false
    }
}

#[cfg(target_os = "windows")]
fn is_web_launcher_entry(path: &str, target_path: &str, arguments: Option<&str>) -> bool {
    let target = if target_path.trim().is_empty() {
        path
    } else {
        target_path
    };
    let target = normalize_app_launch_key(target);
    if target.rsplit('\\').next() != Some("rundll32.exe") {
        return false;
    }
    let Some(arguments) = arguments else {
        return false;
    };
    // Only the known URL module + browser entrypoints + web destination qualify.
    // A URL embedded in arguments to a real application is not enough.
    static URL_LAUNCH: OnceLock<Regex> = OnceLock::new();
    let Some(captures) = URL_LAUNCH.get_or_init(|| Regex::new(
        r#"(?i)^\s*(?:"(?P<quoted>[^"]+)"|(?P<plain>[^",\s]+))\s*,\s*(?:openurl[aw]?|fileprotocolhandler)\s+"?https?://"#
    ).unwrap()).captures(arguments) else { return false };
    let module = captures
        .name("quoted")
        .or_else(|| captures.name("plain"))
        .unwrap()
        .as_str();
    let module = normalize_app_launch_key(module);
    matches!(module.rsplit('\\').next(), Some("url" | "url.dll"))
}

#[cfg(target_os = "windows")]
fn should_skip_web_launcher(
    title: &str,
    path: &str,
    target_path: &str,
    arguments: Option<&str>,
    source: &str,
) -> bool {
    if !is_web_launcher_entry(path, target_path, arguments) {
        return false;
    }
    println!(
        "[AppIndexTrace] 跳过网页启动入口（rundll32 URL 转发）: source={}, title={}, path={}, target_path={}, arguments={}",
        source, title, path, target_path, arguments.unwrap_or_default()
    );
    true
}

#[cfg(target_os = "windows")]
fn append_secondary_apps(
    items: &mut Vec<FileIndex>,
    seen: &mut HashSet<String>,
    apps: &[AppMetadata],
    source: &str,
    path_checks: &mut AppPathChecks,
) {
    for app in apps {
        let (Some(title), Some(path)) = (app.get("title"), app.get("data")) else {
            continue;
        };
        let extension = Path::new(path).extension().and_then(|value| value.to_str());
        if !matches!(extension, Some(value) if ["exe", "rdp", "lnk"].iter().any(|ext| value.eq_ignore_ascii_case(ext)))
        {
            continue;
        }
        let target_path = app
            .get("target_path")
            .map(String::as_str)
            .unwrap_or_default();
        let advertised_shortcut = app
            .get("advertised_shortcut")
            .is_some_and(|value| value == "true");
        if title.trim().is_empty() || path.trim().is_empty() {
            println!(
                "[AppIndexTrace] 跳过快捷方式（字段为空）: source={}, title={}, path={}, target_path={}",
                source, title, path, target_path
            );
            continue;
        }
        if !is_launchable_shortcut_target(target_path) && !advertised_shortcut {
            println!(
                "[AppIndexTrace] 跳过快捷方式（目标类型不支持）: source={}, title={}, path={}, target={}",
                source, title, path, target_path
            );
            continue;
        }
        if advertised_shortcut && !is_launchable_shortcut_target(target_path) {
            println!(
                "[AppIndexTrace] 保留广告式快捷方式（交由 Windows Installer 启动）: source={}, title={}, path={}, target={}",
                source, title, path, target_path
            );
        }
        if is_auxiliary_app_entry(title, target_path) {
            println!(
                "[AppIndexTrace] 跳过快捷方式（标题或目标命中辅助入口）: source={}, title={}, path={}, target={}",
                source, title, path, target_path
            );
            continue;
        }
        let arguments = app.get("arguments").cloned().unwrap_or_default();
        let known = app
            .get("arguments_known")
            .is_some_and(|value| value == "true");
        if should_skip_web_launcher(
            title,
            path,
            target_path,
            known.then_some(arguments.as_str()),
            source,
        ) {
            continue;
        }
        let key = app_entry_key(path, target_path, known.then_some(arguments.as_str()));
        if seen.contains(&key) {
            println!(
                "[AppIndexTrace] 跳过重复入口: source={}, title={}, path={}, target_path={}",
                source, title, path, target_path
            );
            continue;
        }
        if path_checks.should_skip(title, path, target_path, advertised_shortcut, source) {
            continue;
        }
        seen.insert(key);
        let (pinyin, abb) = text_to_pinyin(title);
        items.push(FileIndex {
            title: title.clone(),
            path: path.clone(),
            desc: app_display_desc(target_path, source != "开始菜单"),
            target_path: target_path.to_string(),
            arguments,
            icon: read_icon_to_base64(path.clone()),
            pinyin,
            abb,
            file_type: "app".to_string(),
            ..Default::default()
        });
    }
}

/// Use exactly the same encoding for every source, including empty arguments.
#[cfg(target_os = "windows")]
fn app_entry_key(path: &str, target_path: &str, arguments: Option<&str>) -> String {
    let target = if !target_path.trim().is_empty() {
        target_path
    } else if !is_lnk_name(path) && !path.to_ascii_lowercase().starts_with("shell:") {
        path
    } else {
        ""
    };
    if !target.is_empty() && !target.contains('�') {
        if let Some(arguments) = arguments {
            return serde_json::to_string(&("target", normalize_app_launch_key(target), arguments))
                .unwrap();
        }
    }
    serde_json::to_string(&("entry", normalize_app_launch_key(path))).unwrap()
}

pub fn create_app_index_to_sql(app_handle: AppHandle) -> Result<(), String> {
    static APP_INDEX_REBUILD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _rebuild_guard = APP_INDEX_REBUILD_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "应用索引重建锁异常".to_string())?;
    let config = config::Config::read_local_config().unwrap().base;
    let mut index_db = IndexSQL::new();

    #[cfg(target_os = "macos")]
    let result = {
        let mut result = Vec::new();
        result.extend(get_apps_with_depth(
            "/System/Applications",
            true,
            true,
            false,
            &config.local_app_search_exclude_paths,
        ));
        result.extend(get_apps_with_depth(
            "/Applications/",
            true,
            true,
            false,
            &config.local_app_search_exclude_paths,
        ));
        result.extend(get_apps_with_depth(
            "/System/Volumes/Preboot/Cryptexes/App/System/Applications",
            true,
            true,
            false,
            &config.local_app_search_exclude_paths,
        ));
        result
    };

    #[cfg(target_os = "windows")]
    let items = {
        let registered = crate::api::windows_apps::get_all_app()?;
        let (mut start_menu_apps, mut desktop_apps) = collect_secondary_apps(&app_handle);
        let mut path_checks = AppPathChecks::default();
        // Remove stale links before they can supply arguments to AppsFolder.
        for (apps, source) in [
            (&mut start_menu_apps, "开始菜单"),
            (&mut desktop_apps, "桌面"),
        ] {
            apps.retain(|app| {
                !path_checks.should_skip(
                    app.get("title").map(String::as_str).unwrap_or_default(),
                    app.get("data").map(String::as_str).unwrap_or_default(),
                    app.get("target_path")
                        .map(String::as_str)
                        .unwrap_or_default(),
                    app.get("advertised_shortcut")
                        .is_some_and(|value| value == "true"),
                    source,
                )
            });
        }
        // Only an AppsFolder entry that survives validation and de-duplication may
        // suppress its executable directory during portable discovery. Otherwise a
        // rejected launcher/updater entry could hide a valid main executable nearby.
        let mut registered_app_dirs = Vec::new();
        let mut seen: HashSet<String> = index_db
            .list_custom_app_indexes()
            .map_err(|error| format!("读取手动应用失败: {error}"))?
            .iter()
            .map(|app| app_entry_key(&app.path, &app.target_path, Some(&app.arguments)))
            .collect();
        let mut seen_app_names = HashSet::new();
        let mut items = Vec::new();

        // AppsFolder is Windows' user-facing application catalog. Insert it first so
        // registered applications win over portable-directory duplicates.
        for app in registered {
            let title = app.name.trim();
            let start = app.start.trim();
            if title.is_empty() || start.is_empty() {
                continue;
            }
            let auxiliary_target = if app.executable.is_empty() {
                start
            } else {
                app.executable.as_str()
            };
            if is_auxiliary_app_entry(title, auxiliary_target) {
                continue;
            }
            let arguments = app.arguments.clone().or_else(|| {
                associated_shortcut_arguments(
                    title,
                    &app.executable,
                    &start_menu_apps,
                    &desktop_apps,
                )
            });
            if app.arguments.is_none() {
                println!(
                    "[AppIndexTrace] AppsFolder 参数状态: title={}, target={}, associated={}",
                    title,
                    app.executable,
                    arguments.is_some()
                );
            }
            if should_skip_web_launcher(
                title,
                start,
                &app.executable,
                arguments.as_deref(),
                "AppsFolder",
            ) {
                continue;
            }
            let key = app_entry_key(start, &app.executable, arguments.as_deref());
            if key.is_empty() || seen.contains(&key) {
                continue;
            }
            if path_checks.should_skip(title, start, &app.executable, false, "AppsFolder") {
                continue;
            }
            seen.insert(key);
            let (pinyin, abb) = text_to_pinyin(title);
            let mut icon = if !app.icon.is_empty() {
                read_file_to_base64(&app.icon).unwrap_or_default()
            } else {
                String::new()
            };
            if icon.is_empty() {
                // Read the registered item's own icon first, without a .lnk
                // overlay; this also handles Run/Control Panel/Explorer.
                icon = read_icon_to_base64(start.to_string());
            }
            if icon.is_empty() && !app.executable.is_empty() {
                icon = read_icon_to_base64(app.executable.clone());
            }
            seen_app_names.insert(normalize_app_product_key(title));
            if let Some(directory) = registered_app_directory(&app.executable) {
                if !registered_app_dirs.contains(&directory) {
                    registered_app_dirs.push(directory);
                }
            }
            items.push(FileIndex {
                title: title.to_string(),
                path: start.to_string(),
                desc: String::new(),
                target_path: app.executable,
                arguments: arguments.unwrap_or_default(),
                icon,
                pinyin,
                abb,
                file_type: "app".to_string(),
                ..Default::default()
            });
        }

        append_secondary_apps(
            &mut items,
            &mut seen,
            &start_menu_apps,
            "开始菜单",
            &mut path_checks,
        );
        append_secondary_apps(
            &mut items,
            &mut seen,
            &desktop_apps,
            "桌面",
            &mut path_checks,
        );

        let mut portable_candidates = Vec::new();
        for portable_root in config
            .local_app_search_paths
            .iter()
            .filter(|path| Path::new(path).is_dir())
        {
            println!("扫描便携应用目录: {}", portable_root);
            for app in get_apps_with_depth(
                portable_root,
                true,
                true,
                true,
                &config.local_app_search_exclude_paths,
            ) {
                let (Some(title), Some(path)) = (app.get("title"), app.get("data")) else {
                    continue;
                };
                let path_obj = Path::new(path);
                if !path_obj
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
                    || is_portable_auxiliary_app(title, path)
                    || is_filtered_app(path, title)
                    || is_under_registered_app_dir(path, &registered_app_dirs)
                {
                    continue;
                }
                portable_candidates.push((
                    path.clone(),
                    title.clone(),
                    app.get("target_path")
                        .cloned()
                        .unwrap_or_else(|| path.clone()),
                ));
            }
        }
        portable_candidates.sort_by_key(|(path, _, _)| {
            (
                Path::new(path).components().count(),
                normalize_app_launch_key(path),
            )
        });
        let mut portable_names = HashSet::new();
        for (path, title, target_path) in portable_candidates {
            let key = app_entry_key(&path, &target_path, Some(""));
            let product_key = normalize_app_product_key(
                Path::new(&path)
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or(&title),
            );
            let family_key = portable_family_key(&path, &title);
            if key.is_empty()
                || product_key.is_empty()
                || family_key.is_empty()
                || seen.contains(&key)
                || seen_app_names.contains(&product_key)
                || path_checks.should_skip(&title, &path, &target_path, false, "便携应用")
                || !portable_names.insert(family_key)
            {
                continue;
            }
            seen.insert(key);
            seen_app_names.insert(product_key);
            let (pinyin, abb) = text_to_pinyin(&title);
            items.push(FileIndex {
                title,
                path: path.clone(),
                desc: app_display_desc(&target_path, true),
                target_path,
                icon: read_icon_to_base64(path),
                pinyin,
                abb,
                file_type: "app".to_string(),
                ..Default::default()
            });
        }
        println!(
            "[AppIndexTrace] 文件存在检查完成: unique_paths={}, cache_hits={}, skipped_entries={}, elapsed_ms={:.3}",
            path_checks.missing.len(), path_checks.cache_hits, path_checks.skipped,
            path_checks.elapsed.as_secs_f64() * 1000.0
        );
        items
    };

    #[cfg(target_os = "macos")]
    let items = result
        .into_iter()
        .map(|app| {
            let title = app.get("title").unwrap().to_string();
            let (pinyin, abb) = text_to_pinyin(&title);
            let local_icon_file = vec!["日历", "迁移助理", "Photo Booth", "系统信息", "系统设置"];
            let icon_file_path = if !local_icon_file.contains(&title.as_str()) {
                read_app_info(&app.get("data").unwrap())
            } else {
                format!("icons/{}.png", title)
            };
            let icon = read_icns_to_base64(&icon_file_path).unwrap_or_default();
            FileIndex {
                title,
                path: app.get("data").unwrap().to_string(),
                desc: app.get("desc").unwrap().to_string(),
                target_path: app.get("target_path").cloned().unwrap_or_default(),
                icon,
                pinyin,
                abb,
                ..Default::default()
            }
        })
        .collect();

    let discovered = items.len();
    println!("应用索引扫描完成，准备写入 {discovered} 条自动应用记录");
    let inserted = index_db
        .replace_discovered_app_indexes(items)
        .map_err(|error| format!("应用索引重建失败: {error}"))?;
    println!("应用索引重建完成：扫描 {discovered} 条，实际写入 {inserted} 条自动应用记录");
    if let Some(window) = app_handle.get_window("skylark") {
        let _ = window.emit("app_index_complete", discovered);
    }
    config::save_index_initialization_flags(Some(true), None)
        .map_err(|error| format!("保存应用索引初始化状态失败: {error}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn normalize_app_name(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['_', '-'], " ")
}

#[cfg(target_os = "windows")]
fn normalize_app_product_key(value: &str) -> String {
    let normalized = normalize_app_name(value);
    let compact = normalized.split_whitespace().collect::<String>();
    let compact = compact.trim_end_matches("32").trim_end_matches("64");
    let bytes = compact.as_bytes();
    let mut cut = compact.len();
    let mut digit_count = 0;
    for index in (0..bytes.len()).rev() {
        if bytes[index].is_ascii_digit() || bytes[index] == b'.' {
            if bytes[index].is_ascii_digit() {
                digit_count += 1;
            }
            cut = index;
        } else {
            break;
        }
    }
    if digit_count >= 2 && cut > 3 {
        compact[..cut].to_string()
    } else {
        compact.to_string()
    }
}

#[cfg(target_os = "windows")]
fn portable_family_key(path: &str, title: &str) -> String {
    let parent = Path::new(path).parent();
    let parent_key = parent
        .into_iter()
        .flat_map(|path| path.components())
        .filter_map(|component| component.as_os_str().to_str())
        .filter(|component| !is_version_directory_name(component))
        .map(normalize_app_product_key)
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>()
        .join("\\");
    let executable_key = normalize_app_product_key(
        Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(title),
    );
    format!("{parent_key}|{executable_key}")
}

#[cfg(target_os = "windows")]
fn is_version_directory_name(value: &str) -> bool {
    let value = value.trim().trim_start_matches(['v', 'V']);
    value.contains('.')
        && value.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
        })
}

#[cfg(target_os = "windows")]
fn is_under_registered_app_dir(path: &str, registered_dirs: &[String]) -> bool {
    let path = normalize_app_launch_key(path);
    if path.is_empty() {
        return false;
    }
    registered_dirs
        .iter()
        .any(|directory| path == *directory || path.starts_with(&format!("{directory}\\")))
}

#[cfg(target_os = "windows")]
fn registered_app_directory(executable: &str) -> Option<String> {
    let executable = executable.trim();
    if executable.is_empty() {
        return None;
    }
    Path::new(executable)
        .parent()
        .map(|parent| normalize_app_launch_key(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
}

#[cfg(target_os = "windows")]
fn is_auxiliary_app_value(value: &str) -> bool {
    const AUXILIARY: &[&str] = &[
        "agent",
        "external",
        "service",
        "svr",
        "helper",
        "launcher",
        "swap",
        "crashpad",
        "updater",
        "uninstaller",
        "uninstall",
        "uninst",
        "unins",
        "readme",
        "old",
        "runtime",
        "worker",
        "bootstrap",
    ];
    let lower = value.to_ascii_lowercase();
    if [
        "qqmusicup",
        "qmdriverhelperx64",
        "startdesktopprojection32",
        "startdesktopprojectionforxp",
        "qmbrowser",
        "qmweiyun",
        "desktopdynamiclyric",
        "qmdesktopanimation",
    ]
    .contains(&lower.as_str())
    {
        return true;
    }
    let tokens = lower.split(|c: char| !c.is_ascii_alphanumeric());
    if tokens.clone().any(|token| AUXILIARY.contains(&token)) {
        return true;
    }
    // Also catch vendor naming such as QQMusicUp or ProductAgent without
    // treating an unrelated product like ServerManager as auxiliary.
    [
        "agent",
        "external",
        "service",
        "svr",
        "helper",
        "launcher",
        "updater",
        "swap",
        "uninst",
        "unins",
        "musicup",
        "driverhelper",
        "desktopdynamiclyric",
        "desktopanimation",
        "projection",
        "qmbrowser",
        "qmweiyun",
    ]
    .iter()
    .any(|suffix| lower.len() > suffix.len() + 3 && lower.ends_with(suffix))
}

#[cfg(target_os = "windows")]
fn contains_strong_auxiliary_word(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("卸载")
        || lower.contains("解除安装")
        || lower.contains("uninstall")
        || lower.contains("uninstaller")
        || lower.contains("unins")
}

#[cfg(target_os = "windows")]
fn is_user_visible_auxiliary_app(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    is_auxiliary_app_value(title)
        || contains_strong_auxiliary_word(title)
        || [
            "网站",
            "主页",
            "设置",
            "配置",
            "使用说明",
            "帮助",
            "意见反馈",
            "打字入门",
            "皮肤下载",
            "细胞词库",
            "检查更新",
            "release notes",
            "manual",
            "documentation",
            "help",
            "installer",
            "install",
        ]
        .iter()
        .any(|word| lower.contains(word))
}

#[cfg(target_os = "windows")]
fn is_launchable_shortcut_target(target: &str) -> bool {
    let target = target.trim().trim_matches('"');
    if target.is_empty() {
        return true;
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("shell:")
        || lower.starts_with("::{")
    {
        return lower.starts_with("shell:") || lower.starts_with("::{");
    }
    matches!(
        Path::new(target)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("exe" | "com" | "msc" | "bat" | "cmd" | "rdp")
    )
}

#[cfg(target_os = "windows")]
fn is_explicit_system_entry(title: &str, target: &str) -> bool {
    let normalized = title.trim().to_ascii_lowercase();
    let known_title = [
        "命令提示符",
        "command prompt",
        "windows powershell",
        "powershell",
        "运行",
        "run",
        "控制面板",
        "control panel",
        "文件资源管理器",
        "file explorer",
        "任务管理器",
        "task manager",
    ]
    .iter()
    .any(|value| normalized == *value);
    if known_title {
        return true;
    }
    let target_lower = target.trim().to_ascii_lowercase();
    target_lower.starts_with("shell:appsfolder\\windows.immersivecontrolpanel")
}

#[cfg(target_os = "windows")]
fn is_auxiliary_app_entry(title: &str, target: &str) -> bool {
    if is_explicit_system_entry(title, target) {
        return false;
    }
    let target_lower = target.trim().to_ascii_lowercase();
    let is_system_settings =
        target_lower.starts_with(r"shell:appsfolder\windows.immersivecontrolpanel");
    let is_msconfig = Path::new(target)
        .file_stem()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("msconfig"));
    let title_lower = title.to_ascii_lowercase();
    if (title_lower.contains("设置") && is_system_settings)
        || (title_lower.contains("配置") && is_msconfig)
    {
        return false;
    }
    is_user_visible_auxiliary_app(title)
        || Path::new(target)
            .file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(contains_strong_auxiliary_word)
}

#[cfg(target_os = "windows")]
fn is_auxiliary_shortcut(title: &str, target: &str) -> bool {
    is_auxiliary_app_entry(title, target)
}

#[cfg(target_os = "windows")]
fn is_portable_auxiliary_app(title: &str, path: &str) -> bool {
    is_auxiliary_app_value(title)
        || Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(is_auxiliary_app_value)
}

#[cfg(all(test, target_os = "windows"))]
mod app_scan_tests {
    use super::{
        app_display_desc, is_auxiliary_app_entry, is_launchable_shortcut_target,
        is_portable_auxiliary_app, is_under_registered_app_dir, is_user_visible_auxiliary_app,
        is_web_launcher_entry, local_app_file_path, matching_working_directory_executable,
        normalize_app_product_key, portable_family_key, registered_app_directory, AppPathChecks,
    };
    use super::{app_entry_key, append_secondary_apps, associated_shortcut_arguments, AppMetadata};
    use crate::utils::database::FileIndex;
    use std::collections::HashSet;

    fn shortcut(title: &str, target: &str, arguments: Option<&str>) -> AppMetadata {
        AppMetadata::from([
            ("title".to_string(), title.to_string()),
            ("data".to_string(), format!(r"C:\Start Menu\{title}.lnk")),
            ("target_path".to_string(), target.to_string()),
            (
                "arguments".to_string(),
                arguments.unwrap_or_default().to_string(),
            ),
            (
                "arguments_known".to_string(),
                arguments.is_some().to_string(),
            ),
        ])
    }

    #[test]
    fn all_sources_share_the_same_key_including_empty_arguments() {
        let target = r"E:\Games\WeGame\wegame.exe";
        assert_eq!(
            app_entry_key(&format!(r"shell:AppsFolder\{target}"), target, Some("")),
            app_entry_key(
                r"C:\Start Menu\WeGame.lnk",
                "e:/games/wegame/WEGAME.EXE",
                Some("")
            ),
        );
        assert_eq!(
            app_entry_key(target, "", Some("")),
            app_entry_key(target, target, Some(""))
        );
    }

    #[test]
    fn rundll32_browser_entries_are_filtered_by_module_export_and_web_destination() {
        let target = r"C:\WINDOWS\system32\rundll32.exe";
        for arguments in [
            r#"url,OpenURL "https://terra.im/gl/?cid=19339&oid=24765&v=3&utm_source=376&utm_medium=cpi&utm_campaign=The-Wandering-Village-v0.11.11a&date=20250614&z=1""#,
            r#"url,OpenURL "https://terra.im/gl/?cid=19339&oid=19705&v=3&utm_source=376&utm_medium=cpi&utm_campaign=The-Wandering-Village-v0.11.11a&date=20250614&z=1""#,
            r#"URL.DLL,OpenURL https://example.com"#,
            r#" "C:\Windows\System32\url.dll" , FileProtocolHandler "http://example.com/""#,
            "url.dll,OpenURLW\thttps://example.com/",
        ] {
            assert!(
                is_web_launcher_entry("shell:AppsFolder\\AutoGenerated", target, Some(arguments)),
                "{arguments}"
            );
            assert!(
                is_web_launcher_entry("rundll32.exe", "", Some(arguments)),
                "{arguments}"
            );
            assert!(
                is_web_launcher_entry(&format!("shell:AppsFolder\\{target}"), "", Some(arguments)),
                "{arguments}"
            );
            assert!(
                is_web_launcher_entry("", r#""%windir%\System32\rundll32.exe""#, Some(arguments)),
                "{arguments}"
            );
        }
        assert!(!is_web_launcher_entry("", target, None));
        for arguments in [
            "",
            "shell32.dll,Control_RunDLL appwiz.cpl",
            "shell32.dll,Control_RunDLL https://example.com",
            "other.dll,OpenURL https://example.com",
            "url.dll,Other https://example.com",
            "url.dll,OpenURLHelper https://example.com",
            "url.dll,OpenURL",
            "url.dll,OpenURL file:///C:/Tool.exe",
            "url.dll,OpenURL C:\\Apps\\Tool.exe",
            "url.dll,OpenURL not-http://example.com",
        ] {
            assert!(
                !is_web_launcher_entry("", target, Some(arguments)),
                "{arguments}"
            );
        }
        assert!(!is_web_launcher_entry(
            "",
            r"C:\Apps\GameLauncher.exe",
            Some("url,OpenURL https://example.com")
        ));
        assert!(!is_web_launcher_entry(
            "",
            target,
            Some("custom.dll,Launch --homepage https://example.com")
        ));
    }

    #[test]
    fn webpage_shortcuts_are_skipped_before_dedup_and_icon_reads() {
        let target = r"C:\Windows\System32\rundll32.exe";
        let arguments = "url,OpenURL https://example.com";
        for source in ["开始菜单", "桌面"] {
            let mut items = Vec::new();
            let mut seen = HashSet::new();
            let mut checks = AppPathChecks::default();
            append_secondary_apps(
                &mut items,
                &mut seen,
                &[
                    shortcut("World of Warships", target, Some(arguments)),
                    shortcut("World of Tanks", target, Some(arguments)),
                ],
                source,
                &mut checks,
            );
            assert!(items.is_empty());
            assert!(seen.is_empty());
            assert!(checks.missing.is_empty());
        }
    }

    #[test]
    fn application_description_only_displays_ordinary_paths_for_visible_sources() {
        let target = r"D:\Apps\工具 with spaces\Tool.exe";
        assert_eq!(app_display_desc(target, true), target);
        assert!(app_display_desc(target, false).is_empty());
        for value in [
            "",
            "::{2559A1F3-21D7-11D4-BDAF-00C04F60B9F0}",
            r"shell:AppsFolder\Tool",
            r"shell:AppsFolder\C:\Apps\Tool.exe",
            r"C:\Installer\Tool.ico",
        ] {
            assert!(app_display_desc(value, true).is_empty(), "{value}");
        }
    }

    #[test]
    fn start_menu_and_desktop_keep_same_target_but_different_descriptions() {
        let directory = std::env::temp_dir().join(format!(
            "lark-source-description-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let link = directory.join("Tool.lnk");
        let target = directory.join("Tool.exe");
        std::fs::write(&link, []).unwrap();
        std::fs::write(&target, []).unwrap();
        let target = target.to_string_lossy().into_owned();
        let mut app = shortcut("Tool", &target, Some(""));
        app.insert("data".to_string(), link.to_string_lossy().into_owned());
        app.insert("desc".to_string(), "display-only".to_string());
        for (source, desc) in [("开始菜单", ""), ("桌面", target.as_str())] {
            let mut items = Vec::new();
            append_secondary_apps(
                &mut items,
                &mut HashSet::new(),
                &[app.clone()],
                source,
                &mut AppPathChecks::default(),
            );
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].target_path, target);
            assert_eq!(items[0].desc, desc);
        }
        std::fs::remove_file(link).unwrap();
        std::fs::remove_file(target).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn launch_keys_preserve_argument_case_quotes_and_spaces() {
        let path = r"C:\Windows\System32\cmd.exe";
        let keys: HashSet<_> = [
            "",
            "/K nodevars.bat",
            "/k nodevars.bat",
            "/K  nodevars.bat",
            "/K \"nodevars.bat\"",
        ]
        .iter()
        .map(|args| app_entry_key(path, path, Some(args)))
        .collect();
        assert_eq!(keys.len(), 5);
    }

    #[test]
    fn unknown_arguments_and_shell_only_entries_use_their_own_path() {
        let target = r"C:\Windows\System32\cmd.exe";
        assert_ne!(
            app_entry_key("shell:AppsFolder\\cmd", target, None),
            app_entry_key(target, target, Some(""))
        );
        assert_ne!(
            app_entry_key(r"C:\Menu\Run.lnk", "", Some("")),
            app_entry_key(r"D:\Menu\Run.lnk", "", Some(""))
        );
    }

    #[test]
    fn shortcut_association_requires_matching_title_target_and_consistent_known_arguments() {
        let target = r"E:\Games\WeGame\wegame.exe";
        let matching = shortcut("WeGame", target, Some(""));
        assert_eq!(
            associated_shortcut_arguments("wegame", target, &[matching.clone()], &[]),
            Some(String::new())
        );
        assert_eq!(
            associated_shortcut_arguments("Other", target, &[matching.clone()], &[]),
            None
        );
        assert_eq!(
            associated_shortcut_arguments("WeGame", r"D:\other.exe", &[matching.clone()], &[]),
            None
        );
        assert_eq!(
            associated_shortcut_arguments(
                "WeGame",
                target,
                &[matching.clone()],
                &[shortcut("WeGame", target, Some("/alternate"))]
            ),
            None
        );
        assert_eq!(
            associated_shortcut_arguments(
                "WeGame",
                target,
                &[matching],
                &[shortcut("WeGame", target, None)]
            ),
            None
        );
    }

    #[test]
    fn registered_duplicate_keeps_appsfolder_path_and_icon() {
        let target = r"E:\Games\WeGame\wegame.exe";
        let path = format!(r"shell:AppsFolder\{target}");
        let mut seen = HashSet::from([app_entry_key(&path, target, Some(""))]);
        let mut items = vec![FileIndex {
            path: path.clone(),
            icon: "registered-icon".to_string(),
            ..Default::default()
        }];
        append_secondary_apps(
            &mut items,
            &mut seen,
            &[shortcut("WeGame", target, Some(""))],
            "开始菜单",
            &mut AppPathChecks::default(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, path);
        assert_eq!(items[0].icon, "registered-icon");
    }

    #[test]
    fn virtual_shell_entries_deduplicate_across_localized_titles() {
        let target = "::{2559A1F3-21D7-11D4-BDAF-00C04F60B9F0}";
        let shortcut = shortcut("Run", target, Some(""));
        let arguments = associated_shortcut_arguments("运行", target, &[shortcut.clone()], &[]);
        assert_eq!(arguments.as_deref(), Some(""));
        let path = r"shell:AppsFolder\Microsoft.Windows.Shell.RunDialog";
        let mut seen = HashSet::from([app_entry_key(path, target, arguments.as_deref())]);
        let mut items = vec![FileIndex {
            path: path.to_string(),
            ..Default::default()
        }];
        append_secondary_apps(
            &mut items,
            &mut seen,
            &[shortcut],
            "开始菜单",
            &mut AppPathChecks::default(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, path);
    }

    #[test]
    fn environment_targets_match_expanded_paths() {
        let windows_dir = std::env::var("WINDIR").unwrap();
        assert_eq!(
            app_entry_key("cmd", r"%windir%\System32\cmd.exe", Some("")),
            app_entry_key("cmd", &format!(r"{windows_dir}\System32\cmd.exe"), Some(""))
        );
    }

    #[test]
    fn product_keys_keep_jetbrains_series_separate() {
        assert_eq!(normalize_app_product_key("PyCharm 2026.1"), "pycharm");
        assert_eq!(normalize_app_product_key("RustRover2026.1"), "rustrover");
        assert_ne!(
            normalize_app_product_key("PyCharm 2026.1"),
            normalize_app_product_key("RustRover2026.1")
        );
    }

    #[test]
    fn family_keys_collapse_version_directories_but_keep_jetbrains_products_separate() {
        assert_eq!(
            portable_family_key(r"D:\App\Quark\quark.exe", "quark"),
            portable_family_key(r"D:\App\Quark\7.1.7.975\quark.exe", "quark")
        );
        assert_ne!(
            portable_family_key(
                r"D:\App\JetBrains\PyCharm 2026.1\pycharm.exe",
                "PyCharm 2026.1"
            ),
            portable_family_key(
                r"D:\App\JetBrains\RustRover2026.1\rustrover.exe",
                "RustRover2026.1"
            )
        );
    }

    #[test]
    fn registered_app_directory_suppresses_internal_executables_but_not_siblings() {
        let registered_dirs = vec![r"d:\app\qqmusic".to_string()];
        assert!(is_under_registered_app_dir(
            r"D:\App\QQMusic\QQMusicUp.exe",
            &registered_dirs
        ));
        assert!(is_under_registered_app_dir(
            r"D:\App\QQMusic\qmbrowser\qmbrowser.exe",
            &registered_dirs
        ));
        assert!(!is_under_registered_app_dir(
            r"D:\App\JetBrains\RustRover2026.1\rustrover.exe",
            &registered_dirs
        ));
        assert!(!is_under_registered_app_dir(
            r"D:\App\QQMusic2\player.exe",
            &registered_dirs
        ));
    }

    #[test]
    fn user_visible_entries_ignore_auxiliary_words_in_the_target_filename() {
        assert!(!is_user_visible_auxiliary_app("Reasonix"));
        assert!(is_portable_auxiliary_app(
            "Reasonix",
            r"D:\App\Reasonix\reasonix-launcher.exe"
        ));
        assert!(is_user_visible_auxiliary_app("Reasonix Updater"));
    }

    #[test]
    fn shortcut_auxiliary_filter_keeps_real_launchers_and_system_entries() {
        assert!(is_auxiliary_app_entry(
            "WeGame卸载",
            r"E:\Games\WeGame\uninstall_complete.exe"
        ));
        assert!(is_auxiliary_app_entry(
            "WeGame主页",
            r"E:\Games\WeGame\WeGame.exe"
        ));
        assert!(!is_auxiliary_app_entry(
            "WeGame",
            r"E:\Games\WeGame\WeGameLauncher.exe"
        ));
        assert!(!is_auxiliary_app_entry(
            "Windows 设置",
            r"shell:AppsFolder\windows.immersivecontrolpanel"
        ));
        assert!(!is_auxiliary_app_entry(
            "系统配置",
            r"C:\Windows\System32\msconfig.exe"
        ));
    }

    #[test]
    fn advertised_shortcut_with_non_executable_target_is_kept() {
        let target = r"C:\Users\ystia\AppData\Roaming\Microsoft\Installer\{id}\Charles.ico";
        let mut advertised = shortcut("Charles", target, Some(""));
        let link_path = std::env::temp_dir().join(format!(
            "lark-advertised-path-check-{}.lnk",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&link_path, []).unwrap();
        advertised.insert("data".to_string(), link_path.to_string_lossy().into_owned());
        advertised.insert("advertised_shortcut".to_string(), "true".to_string());
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        append_secondary_apps(
            &mut items,
            &mut seen,
            &[advertised],
            "开始菜单",
            &mut AppPathChecks::default(),
        );
        std::fs::remove_file(link_path).unwrap();
        assert_eq!(items.len(), 1);

        let mut ordinary = shortcut("Charles", target, Some(""));
        ordinary.insert("advertised_shortcut".to_string(), "false".to_string());
        let mut ordinary_items = Vec::new();
        let mut ordinary_seen = HashSet::new();
        append_secondary_apps(
            &mut ordinary_items,
            &mut ordinary_seen,
            &[ordinary],
            "开始菜单",
            &mut AppPathChecks::default(),
        );
        assert!(ordinary_items.is_empty());
    }

    #[test]
    fn local_file_checks_recognize_drive_paths_and_skip_shell_and_network_ids() {
        for value in [
            r"C:\Apps\Tool.exe",
            " c:/Apps/Tool.exe ",
            r#""C:\Apps\Tool.exe""#,
            r"\\?\C:\Apps\Tool.exe",
            r"shell:AppsFolder\C:\Apps\Tool.exe",
        ] {
            assert_eq!(
                local_app_file_path(value).map(|path| path.to_lowercase()),
                Some(r"c:\apps\tool.exe".to_string()),
                "{value}"
            );
        }
        assert_eq!(
            local_app_file_path(r"%windir%\System32\cmd.exe"),
            Some(format!(
                r"{}\System32\cmd.exe",
                std::env::var("WINDIR").unwrap()
            ))
        );
        for value in [
            "",
            r"C:Tool.exe",
            r"Tools\Tool.exe",
            "http://example.com",
            r"shell:AppsFolder\Microsoft.Windows.Shell.RunDialog",
            "::{2559A1F3-21D7-11D4-BDAF-00C04F60B9F0}",
            r"\\server\share\Tool.exe",
            r"\\?\UNC\server\share\Tool.exe",
            r"C:\%LARK_UNRESOLVED_PATH_TEST%\Tool.exe",
        ] {
            assert_eq!(local_app_file_path(value), None, "{value}");
        }
        let mut checks = AppPathChecks::default();
        // A cached mapped-drive classification must bypass filesystem access.
        checks.remote_drives.insert(b'Z', true);
        assert!(checks
            .missing_file(r"Z:\unavailable-share\Tool.exe")
            .is_none());
    }

    #[test]
    fn file_existence_cache_is_shared_within_scan_and_refreshed_next_scan() {
        let path = std::env::temp_dir().join(format!(
            "lark-existing-path-{}.exe",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, []).unwrap();
        let path = path.to_string_lossy().into_owned();
        let mut checks = AppPathChecks::default();
        assert!(!checks.should_skip("Tool", &path, &path, false, "test"));
        assert_eq!(checks.missing.len(), 1);
        assert_eq!(checks.cache_hits, 1);
        std::fs::remove_file(&path).unwrap();
        // This scan reuses the snapshot; the next rebuild checks the file again.
        assert!(checks
            .missing_file(&path.to_uppercase().replace('\\', "/"))
            .is_none());
        assert_eq!(checks.cache_hits, 2);
        assert!(AppPathChecks::default().should_skip("Tool", &path, &path, false, "test"));
    }

    #[test]
    fn stale_shortcuts_and_appsfolder_targets_are_rejected_without_claiming_dedup_key() {
        let path = std::env::temp_dir().join(format!(
            "lark-stale-path-{}.lnk",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, []).unwrap();
        let path = path.to_string_lossy().into_owned();
        let missing_target = format!("{path}.exe");
        let mut app = shortcut("Tool", &missing_target, Some(""));
        app.insert("data".to_string(), path.clone());
        let mut checks = AppPathChecks::default();
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        append_secondary_apps(&mut items, &mut seen, &[app], "test", &mut checks);
        assert!(items.is_empty());
        assert!(seen.is_empty());
        assert!(checks.should_skip(
            "Tool",
            "shell:AppsFolder\\Tool",
            &missing_target,
            false,
            "test"
        ));
        assert!(checks.should_skip(
            "Tool",
            &format!("shell:AppsFolder\\{missing_target}"),
            "",
            false,
            "test"
        ));
        assert!(!checks.should_skip("Charles", &path, &missing_target, true, "test"));
        assert!(!checks.should_skip(
            "运行",
            &path,
            "::{2559A1F3-21D7-11D4-BDAF-00C04F60B9F0}",
            false,
            "test"
        ));
        std::fs::remove_file(&path).unwrap();
        assert!(AppPathChecks::default().should_skip(
            "Charles",
            &path,
            &missing_target,
            true,
            "test"
        ));
    }

    #[test]
    fn working_directory_recovery_requires_one_title_matching_executable() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lark-shortcut-dir-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("AMDPrivacyView.exe");
        std::fs::write(&executable, []).unwrap();
        assert_eq!(
            matching_working_directory_executable("AMD Privacy View", &directory.to_string_lossy()),
            Some(executable.to_string_lossy().into_owned())
        );
        std::fs::write(directory.join("Other.exe"), []).unwrap();
        assert_eq!(
            matching_working_directory_executable("AMD Privacy View", &directory.to_string_lossy()),
            Some(executable.to_string_lossy().into_owned())
        );
        std::fs::write(directory.join("AMDPrivacyView2.exe"), []).unwrap();
        assert_eq!(
            matching_working_directory_executable("AMD Privacy View", &directory.to_string_lossy()),
            Some(executable.to_string_lossy().into_owned())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shortcut_target_filter_rejects_documents_links_and_icon_files() {
        for target in [
            r"D:\APP\Pure Codec\PureSet.htm",
            r"C:\Program Files\Git\ReleaseNotes.html",
            r"D:\APP\SogouInput\Sogou Helper.url",
            r"C:\Windows\System32\appverif.chm",
            r"C:\Users\ystia\AppData\Roaming\Microsoft\Installer\{id}\Charles.ico",
            "http://support.steampowered.com/",
        ] {
            assert!(!is_launchable_shortcut_target(target), "{target}");
        }
        for target in [
            r"C:\Windows\System32\cmd.exe",
            r"C:\Windows\System32\control.exe",
            r"C:\Windows\System32\compmgmt.msc",
            r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
            "",
        ] {
            assert!(is_launchable_shortcut_target(target), "{target}");
        }
    }

    #[test]
    fn shortcut_titles_filter_small_help_and_installation_entries() {
        for (title, target) in [
            ("使用说明", r"D:\APP\Pure Codec\PureSet.htm"),
            (
                "Application Verifier Help",
                r"C:\Windows\System32\appverif.chm",
            ),
            ("GPUView Help", r"C:\Program Files\GPUView.chm"),
            (
                "Visual Studio Installer",
                r"C:\Program Files\Installer\setup.exe",
            ),
            ("检查更新", r"C:\Program Files\CheckForUpdates.exe"),
        ] {
            assert!(is_auxiliary_app_entry(title, target), "{title}");
        }
        assert!(!is_auxiliary_app_entry(
            "Visual Studio",
            r"C:\Program Files\Microsoft Visual Studio\Common7\IDE\devenv.exe"
        ));
    }

    #[test]
    fn registered_directory_normalizes_the_accepted_executable_parent() {
        assert_eq!(
            registered_app_directory(r"D:\App\Reasonix\reasonix-launcher.exe"),
            Some(r"d:\app\reasonix".to_string())
        );
        assert_eq!(registered_app_directory(""), None);
    }

    #[test]
    fn auxiliary_rules_cover_known_background_processes_without_server_manager_false_positive() {
        assert!(is_portable_auxiliary_app(
            "QQMusicUp",
            r"C:\Apps\QQMusicUp.exe"
        ));
        for name in [
            "DesktopDynamicLyric",
            "QMDesktopAnimation",
            "QMDriverHelperx64",
            "QMWeiyun",
            "QQMusicUninst",
            "StartDesktopProjection32",
            "StartDesktopProjectionForXP",
            "qmbrowser",
        ] {
            assert!(is_portable_auxiliary_app(
                name,
                &format!(r"C:\Apps\{name}.exe")
            ));
        }
        assert!(is_portable_auxiliary_app(
            "Quark PWA Launcher",
            r"C:\Apps\quark_pwa_launcher.exe"
        ));
        assert!(!is_portable_auxiliary_app(
            "ServerManager",
            r"C:\Apps\ServerManager.exe"
        ));
    }
}

#[cfg(target_os = "windows")]
fn normalize_app_launch_key(value: &str) -> String {
    expand_app_environment(value)
        .trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_lowercase()
}

#[cfg(target_os = "windows")]
fn expand_app_environment(value: &str) -> String {
    static ENV_PATTERN: OnceLock<Regex> = OnceLock::new();
    ENV_PATTERN
        .get_or_init(|| Regex::new(r"%([^%]+)%").unwrap())
        .replace_all(value, |captures: &regex::Captures<'_>| {
            std::env::var(&captures[1]).unwrap_or_else(|_| captures[0].to_string())
        })
        .into_owned()
}

#[tauri::command(rename_all = "camelCase")]
pub fn add_custom_app_index(app_path: String, app_name: String) -> Result<(), String> {
    let app_path = app_path.trim().trim_matches('"');
    let app_name = app_name.trim();
    if app_name.is_empty() {
        return Err("应用名称不能为空".to_string());
    }
    let path = Path::new(app_path);
    if !path.is_file() {
        return Err("应用路径不存在或不是文件".to_string());
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        return Err("目前仅支持添加 .exe 应用".to_string());
    }
    let canonical_path =
        std::fs::canonicalize(path).map_err(|error| format!("无法规范化应用路径：{error}"))?;
    let canonical_path = canonical_path
        .to_str()
        .ok_or_else(|| "应用路径不是有效文本".to_string())?
        .to_string();
    #[cfg(target_os = "windows")]
    let canonical_path = if let Some(path) = canonical_path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else {
        canonical_path
            .strip_prefix(r"\\?\")
            .unwrap_or(&canonical_path)
            .to_string()
    };

    let (pinyin, abb) = text_to_pinyin(app_name);
    let app = FileIndex {
        title: app_name.to_string(),
        path: canonical_path.clone(),
        desc: canonical_path.clone(),
        target_path: canonical_path.clone(),
        icon: read_icon_to_base64(canonical_path),
        pinyin,
        abb,
        file_type: "app".to_string(),
        ..Default::default()
    };
    let mut index = IndexSQL::new();
    index
        .upsert_custom_app_index(&app)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_custom_app_indexes() -> Result<Vec<FileIndex>, String> {
    IndexSQL::new()
        .list_custom_app_indexes()
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn delete_custom_app_index(id: i64) -> Result<(), String> {
    IndexSQL::new()
        .delete_custom_app_index(id)
        .map_err(|error| error.to_string())
}

pub fn create_file_index_to_sql(app_handle: AppHandle) -> Result<(), String> {
    let config = config::Config::read_local_config().unwrap().base;
    #[cfg(target_os = "macos")]
    {
        let index_db = IndexSQL::new();
        let main_window = app_handle.get_window("skylark").unwrap();
        let home_dir = tauri::api::path::home_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let child_dir = std::fs::read_dir(&home_dir).unwrap();
        // 查找用户主目录下所有文件夹
        for entry in child_dir {
            if let Ok(dir) = entry {
                let dir_path = dir.path().to_str().unwrap().to_string();
                // 过滤隐藏文件
                if !dir.file_name().to_str().expect(".").starts_with(".") {
                    // 递归查找文件夹
                    if dir.path().is_dir() {
                        let skip_paths = config.local_file_search_exclude_paths.clone();
                        let skip_extensions = config.local_file_search_exclude_types.clone();
                        file_scanning(
                            app_handle.clone(),
                            vec![dir_path],
                            skip_paths,
                            skip_extensions,
                        )?;
                    } else {
                        // 文件即入库
                        let title = dir.file_name().to_str().unwrap_or("").to_string();
                        let file_type = dir
                            .path()
                            .extension()
                            .unwrap_or("".as_ref())
                            .to_str()
                            .unwrap()
                            .to_string();
                        let f = FileIndex {
                            title: title.clone(),
                            file_type,
                            path: dir_path,
                            ..Default::default()
                        };
                        main_window.emit("file_index_count", &title).unwrap();
                        index_db.insert_if_not_exist("file", &f).unwrap()
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let roots = config
            .local_file_search_paths
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| Path::new(path).is_dir())
            .collect::<Vec<_>>();
        if roots.is_empty() {
            return Err("没有可用的文件扫描路径".to_string());
        }
        println!("扫描配置目录: {:?}", roots);
        file_scanning(
            app_handle.clone(),
            roots,
            config.local_file_search_exclude_paths.clone(),
            config.local_file_search_exclude_types.clone(),
        )?;
    }
    config::save_index_initialization_flags(None, Some(true))
        .map_err(|error| format!("保存文件索引初始化状态失败: {error}"))?;
    Ok(())
}

pub(crate) fn is_process_running(process_name: &str) -> bool {
    let snapshot: HANDLE = unsafe { CreateToolhelp32Snapshot(0x2, 0) }; // TH32CS_SNAPALL
    let mut process_entry: PROCESSENTRY32 = unsafe { std::mem::zeroed() };
    process_entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(snapshot, &mut process_entry) } == 0 {
        return false;
    }

    loop {
        let _name2 = unsafe { std::ffi::CStr::from_ptr(process_entry.szExeFile.as_ptr()) };
        let name = unsafe { std::ffi::CStr::from_ptr(process_entry.szExeFile.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        if name == process_name {
            return true;
        }
        if unsafe { Process32Next(snapshot, &mut process_entry) } == 0 {
            break;
        }
    }
    false
}

fn open_or_activate_app(process_name: &str, app_name: &str) {
    let current_dir = Path::new(process_name).parent().unwrap();
    let program = process_name.split("\\").last().expect("aa.exe");
    let _window_name = process_name.strip_suffix(".exe").unwrap_or(process_name);

    if is_process_running(program) {
        // 激活窗口
        println!("Process {} is already running.", process_name);
        find_windows_with_partial_title(app_name);
    } else {
        if current_dir
            .to_string_lossy()
            .to_uppercase()
            .contains(r"C:\WINDOWS\SYSTEM32")
        {
            let result = Command::new("cmd")
                .arg("/c")
                .arg("start")
                .raw_arg("\"\"")
                .arg("/d")
                .raw_arg(format!("\"{}\"", current_dir.to_string_lossy()))
                .raw_arg(format!("\"{}\"", process_name))
                // .args(args)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
                .expect("failed to start process");
            println!("打开程序[cmd]:{:?}", result);
        } else {
            let wide_application_name: Vec<u16> = OsStr::new(process_name)
                .encode_wide()
                .chain(Some(0).into_iter())
                .collect();
            let current_dir: Vec<u16> = current_dir
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut startup_info: STARTUPINFOW = unsafe { std::mem::zeroed() };
            startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

            let mut process_info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

            let result = unsafe {
                CreateProcessW(
                    wide_application_name.as_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                    0,
                    ptr::null_mut(),
                    current_dir.as_ptr(),
                    &mut startup_info,
                    &mut process_info,
                )
            };
            println!("打开程序[api]:{:?}", result);
        }
    }
}

unsafe extern "system" fn enum_window_proc(hwnd: HWND, l_param: LPARAM) -> i32 {
    let search_text = l_param as *const u16;
    let search_osstr = OsString::from_wide(std::slice::from_raw_parts(search_text, 20)); // 假设长度为 20
    let binding = search_osstr.to_string_lossy().clone();
    // 无法确定字符串长度，预先添加###作为终止符
    let search_str = binding.split("###").collect::<Vec<_>>()[0];
    // println!("{}",search_str);

    let length = GetWindowTextLengthW(hwnd);
    if length > 0 {
        let mut title = vec![0u16; 256]; // 创建缓冲区
        GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);

        let title_str = OsString::from_wide(&title);
        let title_str = title_str.to_string_lossy().replace("\0", ""); // 剔除不可见字符
        if title_str.to_string().contains(&search_str.to_string()) {
            println!("匹配到应用: {}", title_str.replace(" ", ""));
            // 将 Rust 字符串转换为宽字符
            let wide_window_name: Vec<u16> = OsStr::new(&title_str)
                .encode_wide()
                .chain(std::iter::once(0)) // 添加 null 终止符
                .collect();
            let hwnd = unsafe { FindWindowW(ptr::null_mut(), wide_window_name.as_ptr()) };
            if hwnd.is_null() {
                println!("Window not found.");
            } else {
                // 先恢复窗口（如果被最小化）
                unsafe {
                    ShowWindow(hwnd, SW_RESTORE); // 恢复窗口
                    SetForegroundWindow(hwnd); // 激活窗口
                }
                println!("Activated window for {}", search_str);
            }
            return 0;
        }
    }
    1 // 继续枚举
}
pub(crate) fn find_windows_with_partial_title(partial_title: &str) {
    let partial_title = &(partial_title.to_owned() + "###");
    let wide_partial_title: Vec<u16> = OsStr::new(partial_title)
        .encode_wide()
        .chain(std::iter::once(0)) // 添加 null 终止符
        .collect();

    unsafe {
        EnumWindows(
            Some(enum_window_proc),
            wide_partial_title.as_ptr() as LPARAM,
        );
    }
}

#[test]
#[cfg(target_os = "windows")]
fn app_launch_keys_normalize_case_slashes_and_quotes() {
    assert_eq!(
        normalize_app_launch_key(r#""C:/Apps/Tool.exe""#),
        r"c:\apps\tool.exe"
    );
    assert_eq!(
        normalize_app_launch_key(r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
        r"shell:appsfolder\microsoft.windowscalculator_8wekyb3d8bbwe!app"
    );
}

#[test]
#[cfg(target_os = "windows")]
fn app_scan_exclusions_match_only_the_selected_subtree() {
    let excluded = vec![r"D:\Apps\SDK".to_string()];
    assert!(is_app_scan_path_excluded(r"d:/apps/sdk", &excluded));
    assert!(is_app_scan_path_excluded(
        r"D:\Apps\SDK\tools\bin",
        &excluded
    ));
    assert!(!is_app_scan_path_excluded(r"D:\Apps\SDK2", &excluded));
    assert!(!is_app_scan_path_excluded(r"D:\Apps\Other", &excluded));
}

#[test]
#[allow(unused)]
fn test1() {
    // use chrono::Duration;
    // use std::thread;

    // let process_name = r"C:\windows\system32\notepad.exe";
    // open_or_activate_app(process_name, "记事本");

    let link = lnk::ShellLink::open(
        r"C:\Users\admin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\百度翻译.lnk",
    )
    .expect("无法打开链接");
    // 获取目标路径
    println!("{}", link.working_dir().clone().unwrap_or("aa".to_string()));
    println!(
        "{}",
        link.relative_path().clone().unwrap_or("bb".to_string())
    );
    println!(
        "{}",
        link.link_info()
            .clone()
            .unwrap()
            .local_base_path()
            .clone()
            .unwrap()
    );
    println!(
        "{}",
        read_icon_to_base64(
            r"C:\Users\admin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\百度翻译.lnk"
                .to_string()
        )
    );
    use std::env;
    let home_drive = env::var("HOMEDRIVE").unwrap_or_else(|_| String::from(""));
    let home_path = env::var("HOMEPATH").unwrap_or_else(|_| String::from(""));

    // 组合两个变量
    let home_directory = format!("{}{}", home_drive, home_path);

    println!("用户主目录: {}", home_directory);
}
