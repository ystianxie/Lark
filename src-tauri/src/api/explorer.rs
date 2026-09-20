use crate::config;
use crate::utils::database::{FileIndex, IndexSQL};
use crate::utils::icons;
use crate::utils::string_factory::text_to_pinyin;
use base64::encode;
use base64::engine::{general_purpose, Engine};
use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};
use encoding_rs_io::DecodeReaderBytesBuilder;
use icns::{IconFamily, IconType};
use image::DynamicImage;
use log::{debug, info};
use pinyin::{Pinyin, ToPinyin};
use plist::Value;
use rayon::iter::{ParallelBridge, ParallelIterator};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::ascii::escape_default;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{default, panic};
use tauri::{AppHandle, Emitter, Manager};
use walkdir::{DirEntry, WalkDir};

use std::ffi::{OsStr, OsString};
use std::ops::Index;
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
    let mut fd_output = Command::new("fd")
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
    if let Ok(result) = db.find_by_keyword("file", keyword, offset) {
        return result;
    }
    vec![FileIndex {
        ..Default::default()
    }]
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
        return result;
    }
    vec![FileIndex {
        ..Default::default()
    }]
}

#[derive(Debug)]
enum ApplicationError {
    UnsupportedExtension,
    NotFoundExecutable,
    LnkParseError,
}

// 从 lnk 文件总获取应用配置
fn get_app_from_lnk(lnk_path: &str) -> Result<Vec<String>, ApplicationError> {
    // println!("快捷方式路径：{}",lnk_path);

    let result = panic::catch_unwind(|| {
        if let Ok(res) = lnk::ShellLink::open(lnk_path) {
            return res;
        }
        return Default::default();
    });
    match result {
        Ok(shortcut) => match shortcut.link_info() {
            Some(link_info) => {
                if let [Some(working_dir), Some(target)] =
                    [shortcut.working_dir(), link_info.local_base_path()]
                {
                    if !target.ends_with(".exe") && !target.ends_with("rdp") {
                        return Err(ApplicationError::UnsupportedExtension);
                    }

                    // let mut app = HashMap::from([
                    //     (
                    //         "icon".to_string(),
                    //         path.to_string(),
                    //     ),
                    //     ("title".to_string(),
                    //      Regex::new(r"\.lnk$").unwrap().replace(
                    //          std::path::Path::new(&lnk_path)
                    //              .file_name()
                    //              .unwrap()
                    //              .to_string_lossy()
                    //              .to_string()
                    //              .as_str(),
                    //          "",
                    //      ).to_string()),
                    //     (
                    //         "desc".to_string(),
                    //         target.to_string(),
                    //     ),
                    //     (
                    //         "data".to_string(),
                    //         path.to_string(),
                    //     ),
                    //     ("type".to_string(), "app".to_string()),
                    // ]);
                    let app_title = Regex::new(r"\.lnk$")
                        .unwrap()
                        .replace(
                            Path::new(&lnk_path)
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string()
                                .as_str(),
                            "",
                        )
                        .to_string();
                    println!("应用target:{target}");
                    let app_software_name: Vec<_> = target.rsplitn(2, "\\").collect();
                    let mut app_software_name = app_software_name.index(0).to_string();
                    if !working_dir.ends_with("\\") {
                        app_software_name = "\\".to_string() + &app_software_name
                    }
                    let mut app_path_data = target.to_string();
                    // if working_dir.contains("%HOMEPATH%") || working_dir.contains("%%"){
                    //     app_path_data = target.to_string();
                    // }
                    if target.contains("�") && !app_software_name.contains("�") {
                        app_path_data = working_dir.to_string() + &app_software_name;
                    } else if app_software_name.contains("�") {
                        let extensions: Vec<_> = target.rsplitn(2, ".").collect();
                        let extensions = extensions.index(0);
                        app_software_name = "\\".to_string() + &app_title + "." + extensions;
                        app_path_data = working_dir.to_string() + &app_software_name;
                    }
                    // if app_software_name.contains("�") {
                    //     let extensions: Vec<_> = target.rsplitn(2, ".").collect();
                    //     let extensions = extensions.index(0);
                    //     app_software_name = "\\".to_string() + app_title.as_str() + "." + extensions;
                    // }
                    let app = vec![app_title, app_path_data];
                    // if let Some(arguments) = shortcut.arguments() {
                    //     app.arguments = arguments.to_string();
                    // }
                    Ok(app)
                } else {
                    Err(ApplicationError::NotFoundExecutable)
                }
            }
            _ => Err(ApplicationError::LnkParseError),
        },
        Err(_) => Err(ApplicationError::LnkParseError),
    }
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
    let name = name.to_ascii_lowercase();
    [
        "uninstall",
        "updater",
        "crashpad_handler",
        "uninst",
        "unins00",
        "update",
        "upgrade",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
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
            if include_direct_files && is_auxiliary_app_file(app_name) {
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
                    app_title = title.clone();
                    app_path = path.clone();
                    println!("获取到应用:{app_title}--->{app_path}");
                } else {
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
                if !icons::has_embedded_icon(&app_path) {
                    continue;
                }
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
            ("icon".to_string(), app_path.clone()),
            ("title".to_string(), app_title.clone()),
            ("desc".to_string(), app_path.clone()),
            ("data".to_string(), app_path.clone()),
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
    Ok(encode(&contents))
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
            let base64_image = base64::encode(&img_buffer.data);
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
    let base64_image = base64::encode(buffer.get_ref());
    Ok(base64_image)
}

// 获取文件图标
#[tauri::command]
fn read_icon_to_base64(path: String) -> String {
    if let Ok(buffer) = icons::get_icon(&path, 128) {
        return base64::encode(buffer);
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
    let mut cmd = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R");
        command.arg(path);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", path));
        command
    } else if cfg!(target_os = "linux") {
        let mut command = Command::new("xdg-open");
        command.arg(path).spawn().expect("打开失败！");
        command
    } else {
        panic!("Unsupported OS");
    };

    cmd.spawn().expect("打开失败！");

    #[cfg(target_os = "windows")]
    if let Some(folder_name) = Path::new(path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(str::to_owned)
    {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            activate_explorer_window(&folder_name);
        });
    }

    "打开成功！".to_string()
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
) {
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
            .is_some_and(|ext| skip_extensions.iter().any(|value| {
                value.trim().trim_start_matches('.').eq_ignore_ascii_case(ext)
            }))
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
            Err(error) => {
                eprintln!("无法开始文件索引代次: {error}");
                return;
            }
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
                        eprintln!("写入文件索引失败: {error}");
                        return;
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
            eprintln!("提交文件索引失败: {error}");
        } else if let Some(window) = &main_window {
            let _ = window.emit("file_index_complete", ());
        }
    }
    println!("磁盘扫描完成！")
}

pub fn create_app_index_to_sql(app_handle: AppHandle) {
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
        let registered = crate::api::windows_apps::get_all_app();
        let mut seen = HashSet::new();
        let mut items = Vec::new();

        // AppsFolder is Windows' user-facing application catalog. Insert it first so
        // registered applications win over portable-directory duplicates.
        for app in registered {
            let title = app.name.trim();
            let start = app.start.trim();
            if title.is_empty() || start.is_empty() {
                continue;
            }
            let identity = if app.executable.is_empty() {
                start
            } else {
                app.executable.as_str()
            };
            let key = normalize_app_launch_key(identity);
            if key.is_empty() || !seen.insert(key) {
                continue;
            }
            let (pinyin, abb) = text_to_pinyin(title);
            let icon = if !app.icon.is_empty() {
                read_file_to_base64(&app.icon).unwrap_or_default()
            } else if !app.executable.is_empty() {
                read_icon_to_base64(app.executable.clone())
            } else {
                String::new()
            };
            items.push(FileIndex {
                title: title.to_string(),
                path: start.to_string(),
                desc: app.executable,
                icon,
                pinyin,
                abb,
                file_type: "app".to_string(),
                ..Default::default()
            });
        }

        // AppsFolder does not include every shortcut or standalone executable that users
        // place on the desktop. Scan the current and public desktops non-recursively as
        // a secondary source, after registered applications and before portable roots.
        let mut desktop_roots = Vec::new();
        if let Ok(path) = app_handle.path().desktop_dir() {
            desktop_roots.push(path);
        }
        if let Some(public_dir) = std::env::var_os("PUBLIC") {
            desktop_roots.push(PathBuf::from(public_dir).join("Desktop"));
        }
        desktop_roots.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
        desktop_roots.dedup_by(|left, right| {
            left.to_string_lossy()
                .eq_ignore_ascii_case(&right.to_string_lossy())
        });

        for desktop_root in desktop_roots.iter().filter(|path| path.is_dir()) {
            println!("扫描桌面应用: {}", desktop_root.display());
            for app in get_apps_with_depth(&desktop_root.to_string_lossy(), false, true, false, &[])
            {
                let (Some(title), Some(path)) = (app.get("title"), app.get("data")) else {
                    continue;
                };
                let extension = Path::new(path)
                    .extension()
                    .and_then(|extension| extension.to_str());
                if !matches!(extension, Some(value) if value.eq_ignore_ascii_case("exe")
                    || value.eq_ignore_ascii_case("rdp"))
                {
                    continue;
                }
                let key = normalize_app_launch_key(path);
                if title.trim().is_empty() || key.is_empty() || !seen.insert(key) {
                    continue;
                }
                let (pinyin, abb) = text_to_pinyin(title);
                items.push(FileIndex {
                    title: title.clone(),
                    path: path.clone(),
                    desc: app.get("desc").cloned().unwrap_or_else(|| path.clone()),
                    icon: read_icon_to_base64(path.clone()),
                    pinyin,
                    abb,
                    file_type: "app".to_string(),
                    ..Default::default()
                });
            }
        }
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
                if !Path::new(path)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension.eq_ignore_ascii_case("exe"))
                    .unwrap_or(false)
                {
                    continue;
                }
                let key = normalize_app_launch_key(path);
                if title.trim().is_empty() || key.is_empty() || !seen.insert(key) {
                    continue;
                }
                let (pinyin, abb) = text_to_pinyin(title);
                items.push(FileIndex {
                    title: title.clone(),
                    path: path.clone(),
                    desc: app.get("desc").cloned().unwrap_or_else(|| path.clone()),
                    icon: read_icon_to_base64(path.clone()),
                    pinyin,
                    abb,
                    file_type: "app".to_string(),
                    ..Default::default()
                });
            }
        }
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
                icon,
                pinyin,
                abb,
                ..Default::default()
            }
        })
        .collect();

    let discovered = items.len();
    println!("应用索引扫描完成，准备写入 {discovered} 条自动应用记录");
    match index_db.replace_discovered_app_indexes(items) {
        Ok(inserted) => {
            println!("应用索引重建完成：扫描 {discovered} 条，实际写入 {inserted} 条自动应用记录")
        }
        Err(error) => eprintln!("应用索引重建失败: {error}"),
    }
}

#[cfg(target_os = "windows")]
fn normalize_app_launch_key(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_lowercase()
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

pub fn create_file_index_to_sql(app_handle: AppHandle) {
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
                        file_scanning(app_handle.clone(), &dir_path, skip_paths, skip_extensions);
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
        println!("扫描配置目录: {:?}", roots);
        file_scanning(
            app_handle.clone(),
            roots,
            config.local_file_search_exclude_paths.clone(),
            config.local_file_search_exclude_types.clone(),
        );
    }
}

pub(crate) fn is_process_running(process_name: &str) -> bool {
    let mut snapshot: HANDLE = unsafe { CreateToolhelp32Snapshot(0x2, 0) }; // TH32CS_SNAPALL
    let mut process_entry: PROCESSENTRY32 = unsafe { std::mem::zeroed() };
    process_entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(snapshot, &mut process_entry) } == 0 {
        return false;
    }

    loop {
        let name2 = unsafe { std::ffi::CStr::from_ptr(process_entry.szExeFile.as_ptr()) };
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
    let window_name = process_name.strip_suffix(".exe").unwrap_or(process_name);

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
