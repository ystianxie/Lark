use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use base64::encode;
use base64::engine::general_purpose;
use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};
use encoding_rs_io::DecodeReaderBytesBuilder;
use icns::{IconFamily, IconType};
use image::DynamicImage;
use pinyin::ToPinyin;
use plist::Value;
use rayon::iter::{ParallelBridge, ParallelIterator};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use walkdir::{WalkDir, DirEntry};
use crate::config;
use crate::utils::database::{FileIndex, IndexSQL};

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

    return result;
}

pub fn get_all_app(keyword: &str) -> Vec<HashMap<String, String>> {
    let mut result = Vec::new();
    #[cfg(target_os = "macos")]{
        result.extend(get_app("/System/Applications"));
        result.extend(get_app("/Applications/"));
        result.extend(get_app(
            "/System/Volumes/Preboot/Cryptexes/App/System/Applications",
        ));
    }
    #[cfg(target_os = "windows")]{
        result.extend(get_app("C:\\Program Files"));
    }
    result
}

pub fn get_app(path: &str) -> Vec<HashMap<String, String>> {
    // println!("开始检索目录： {:?}", path);
    let mut applications = Vec::new();
    let entries = fs::read_dir(path).unwrap();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

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
        let file_name = entry.file_name();

        let app_name = match file_name.to_str() {
            Some(name) => name,
            None => continue,
        };
        let mut app_title: String = String::new();
        #[cfg(target_os = "macos")]{
            if !app_name.ends_with(".app") {
                // println!("文件夹：{:?}", entry.path().to_str().unwrap());
                applications.extend(get_app(entry.path().to_str().unwrap()));
                continue;
            }

            app_title = app_name.to_string().replace(".app", "");

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
        #[cfg(target_os = "windows")]{
            if !app_name.ends_with(".exe") {
                continue;
            }
        }
        let map: HashMap<String, String> = HashMap::from([
            (
                "icon".to_string(),
                entry.path().to_str().unwrap().to_string(),
            ),
            ("title".to_string(), app_title.to_string()),
            (
                "desc".to_string(),
                entry.path().to_str().unwrap().to_string(),
            ),
            (
                "data".to_string(),
                entry.path().to_str().unwrap().to_string(),
            ),
            ("type".to_string(), "app".to_string()),
        ]);
        applications.push(map);
    }
    applications
}


#[tauri::command(rename_all = "camelCase")]
pub fn read_file_to_base64(path: &str) -> String {
    let mut file = File::open(path).expect("无法打开文件");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("无法读取文件内容");
    let base64_contents = encode(&contents);
    base64_contents
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_icns_to_base64(path: &str) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let icon_family = IconFamily::read(file).map_err(|e| e.to_string())?;
    let img_index = match path.contains("AirPort Utility.app") {
        true => 2,
        false => 0,
    };
    println!("{:?}::{:?}", path, img_index);
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
        if icon_name.is_empty() {
            icon_name = "AppIcon".to_string();
            let path = Path::new(&icon_name);
            if !path.exists() {
                return "文件不存在！".to_string();
            }
        }
        if !icon_name.contains(".icns") {
            icon_name.push_str(".icns");
        }
        let icon_path = PathBuf::from(&app_path)
            .join("Contents/Resources")
            .join(icon_name);
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
        command.arg("/select,");
        command.arg(path);
        command
    } else if cfg!(target_os = "linux") {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    } else {
        panic!("Unsupported OS");
    };

    cmd.spawn().expect("打开失败！");
    "打开成功！".to_string()
}

#[cfg(target_os = "windows")]
fn get_drives() -> Vec<(String, String)> {
    let mut drives = Vec::new();
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

fn file_scanning(app_handle: AppHandle, root_dir: &str, skip_dirs: Vec<String>, skip_extensions: Vec<String>) {
    fn is_hidden(entry: &DirEntry) -> bool {
        entry.file_name().to_str().map_or(false, |s| s.starts_with('.'))
    }
    fn should_skip_file(entry: &DirEntry, skip_extensions: &[String]) -> bool {
        entry.path().extension().and_then(|ext| ext.to_str()).map_or(false, |ext| skip_extensions.contains(&ext.to_string()))
    }

    fn should_skip_dir(entry: &DirEntry, skip_dirs: &[String]) -> bool {
        let mut is_skip = false;
        is_skip = skip_dirs.iter().any(|dir| entry.path().starts_with(dir));
        if !is_skip {
            is_skip = skip_dirs.iter().any(|dir| {
                if dir.starts_with("*/") {
                    let skip_key = dir.split("/").last().clone().unwrap();
                    entry.file_name().to_str().unwrap().contains(skip_key)
                } else {
                    false
                }
            });
        }
        is_skip
    }

    let skip_extensions_data = Arc::new(skip_extensions);
    let skip_dirs_data = Arc::new(skip_dirs);
    let files = Arc::new(Mutex::new(Vec::new()));
    let main_window = app_handle.get_window("skylark").unwrap();
    let root_dir = root_dir.to_string();
    tauri::async_runtime::spawn(async move {
        WalkDir::new(root_dir).into_iter()
            .filter_entry(move |entry| {
                !is_hidden(entry) && !should_skip_file(entry, &skip_extensions_data) && !should_skip_dir(entry, &skip_dirs_data)
            })
            .filter_map(Result::ok)
            .for_each(|entry| {
                let title = entry.file_name().to_str().unwrap_or("").to_string();
                let file_path = entry.path().display().to_string();
                let file_type = if entry.path().is_dir() {
                    "folder".to_string()
                } else {
                    entry.path().extension().and_then(|ext| ext.to_str()).unwrap_or("").to_string()
                };
                main_window.emit("file_index_count", &title).unwrap();
                let mut files = files.lock().unwrap();
                files.push(FileIndex {
                    title,
                    path: file_path,
                    file_type: file_type.to_string(),
                    ..Default::default()
                })
            });
        let files = files.lock().unwrap().clone();
        let mut index_db = IndexSQL::new();
        index_db.insert_indexes("file", files).unwrap();
    });
}
pub fn create_file_index_to_sql(app_handle: AppHandle) {
    let config = config::Config::read_local_config().unwrap().base;
    let index_db = IndexSQL::new();
    let main_window = app_handle.get_window("skylark").unwrap();
    #[cfg(target_os = "macos")]{
        let home_dir = tauri::api::path::home_dir().unwrap().to_str().unwrap().to_string();
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
                        let file_type = dir.path().extension().unwrap_or("".as_ref()).to_str().unwrap().to_string();
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
}