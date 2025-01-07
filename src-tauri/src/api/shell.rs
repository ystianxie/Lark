#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate objc;
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSAutoreleasePool, NSString};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
use super::clipboard::{ClipboardOperator, ImageDataDB};
use open;
use open::that;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::{path, ptr};
use std::path::Path;
use std::process::{Command, Stdio};
use webbrowser;
use anyhow::Result;
use winapi::um::processthreadsapi::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW};

#[tauri::command(rename_all = "camelCase")]
pub fn run_python_script(script_path: &str, params: Vec<String>) -> HashMap<&str, String> {
    // 使用 `Command` 运行 Python 脚本
    println!("{:?}",script_path);
    let output = Command::new("python")
        .arg(script_path)
        .args(params)
        .output()
        .expect("Failed to execute Python script");
    let mut result = HashMap::new();
    if output.status.success() {
        // 将输出转换为字符串并打印
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Script output:\n{}", stdout);
        result.insert("success", "true".to_string());
        result.insert("data", stdout.to_string());
        return result;
    } else {
        // 如果脚本执行失败，打印错误信息
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Script output:\n{}", stderr);
        result.insert("success", "false".to_string());
        result.insert("data", stderr.to_string());
        return result;
    }
}

#[tauri::command(rename_all = "camelCase")]
#[cfg(target_os = "macos")]
pub fn open_app(app_path: &str, app_name: &str) {
    unsafe {
        let pool: id = NSAutoreleasePool::new(nil);

        // Convert app path to NSString
        let app_path_nsstring = NSString::alloc(nil).init_str(app_path);

        // Get file URL
        let file_url: id = msg_send![class!(NSURL), fileURLWithPath: app_path_nsstring];

        // Open application
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let _: id = msg_send![workspace, openURL: file_url];

        pool.drain();
    }
}
#[tauri::command(rename_all = "camelCase")]
#[cfg(target_os = "windows")]
pub fn open_app(app_path: &str, app_name: &str) {
    let current_dir = Path::new(app_path).parent().unwrap();
    let program = app_path.split("\\").last().expect("aa.exe");
    println!("打开app:{:?}", app_path);
    if crate::api::explorer::is_process_running(program) {
        // 激活窗口
        println!("Process {} is already running.", app_name);
        crate::api::explorer::find_windows_with_partial_title(app_name);
    } else {
        if current_dir.to_string_lossy().to_uppercase().contains(r"C:\WINDOWS\SYSTEM32") {
            let result = Command::new("cmd")
                .arg("/c")
                .arg("start")
                .raw_arg("\"\"")
                .arg("/d")
                .raw_arg(format!("\"{}\"", current_dir.to_string_lossy()))
                .raw_arg(format!("\"{}\"", app_path))
                // .args(args)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
                .expect("failed to start process");
            println!("打开程序[cmd]:{:?}", result);
        } else {
            let wide_application_name: Vec<u16> = OsStr::new(app_path)
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


#[tauri::command(rename_all = "camelCase")]
pub fn open_url(url: &str) {
    // 使用默认浏览器打开 URL
    if webbrowser::open(url).is_ok() {
        println!("成功打开浏览器并访问: {}", url);
    } else {
        println!("打开浏览器失败");
        // open::with(url, "/Applications/Microsoft Edge.app").expect("无法打开浏览器");
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_file(file_path: &str) {
    let path = Path::new(file_path); // 替换为你的文件路径

    // 使用默认应用程序打开文件
    if let Err(e) = that(path) {
        eprintln!("Failed to open file: {}", e);
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn clipboard_control(text: &str, control: &str, paste: bool, data_type: &str) -> Result<String, String> {
    println!("剪贴板控制：{:?}", text);
    if control == "write" {
        if data_type == "file" {
            Ok("暂不支持文件复制".to_string())
        } else if data_type == "image" {
            let img = ImageDataDB { base64: text.to_string(), ..Default::default() };
            let _ = ClipboardOperator::set_image(img);
            Ok("写入剪贴板成功".to_string())
        } else {
            let _ = ClipboardOperator::set_text(text);
            if paste {
                let _ = ClipboardOperator::paste_text(text);
            }
            Ok("写入剪贴板成功".to_string())
        }
    } else {
        // todo 获取剪贴板当前内容
        let content = ClipboardOperator::get_text();
        println!("从剪贴板读取的内容：{:?}", content);
        Ok(content.unwrap())
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_file_icon(file_path: &str) -> Result<String, String> {
    println!("读取文件图标路径：{:?}", file_path);
    let output = Command::new("node")
        .args(["src/api/get_file_icon.js", file_path])
        // .spawn()
        .output()
        .expect("failed to execute `get_file_icon` command");

    if output.status.success() {
        let icon_base64 = fs::read_to_string("temp.txt").expect("Failed to read output file");
        Ok(icon_base64.trim().to_string())
    } else {
        let stderr = format!(
            "stderr: {:?}",
            String::from_utf8(output.stderr.into()).unwrap()
        );
        println!("{:?}", stderr);
        if stderr.contains("MODULE_NOT_FOUND") {
            println!("需要安装file-icon");
            return Ok("".to_string());
        }
        Ok(String::from("Failed to get file icon"))
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn write_txt(file_path: &str, text: &str) -> Result<String, String> {
    if path::Path::new(file_path).exists() {
        let mut file_dir = file_path.replace("\\", "/");
        file_dir = file_dir
            .rsplitn(2, "/")
            .nth(0)
            .unwrap_or_else(|| "")
            .to_string();
        let _ = fs::create_dir_all(file_dir);
    }
    let mut f = fs::File::create(file_path).unwrap();
    let _ = f.write(text.as_bytes());
    println!("写入文件：{:?}", file_path);
    Ok("写入成功".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_txt(file_path: &str) -> Result<String, String> {
    if path::Path::new(file_path).exists() {
        let content = fs::read_to_string(file_path).expect("Failed to read file");
        Ok(content)
    } else {
        Ok(String::from("File not found"))
    }
}
#[tauri::command(rename_all = "camelCase")]
pub fn append_txt(file_path: &str, text: &str) -> Result<String, String> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(file_path)
        .expect("cannot open file");
    file.write_all(text.as_bytes()).expect("write failed");
    println!("数据追加成功");
    Ok("数据追加成功".to_string())
}

#[test]
fn test() {
    // let file_path = "/System/Applications/Utilities/Migration Assistant.app";
    // let base64 = get_file_icon(file_path).unwrap();
    // println!("{}", base64)
    let output = run_python_script("D:/Project/Lark/src-tauri/target/debug/config/lark/data/plugins/PrettyPostman/str2json.py",vec![]);
    print!("{:?}",output);
}