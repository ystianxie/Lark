#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate objc;
use super::clipboard::{ClipboardOperator, ImageDataDB};
use anyhow::Result;
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSAutoreleasePool, NSString};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
use open;
use open::that;
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{path, ptr};
use webbrowser;
use winapi::um::processthreadsapi::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW};

#[tauri::command(rename_all = "camelCase")]
pub fn run_python_script(script_path: &str, params: Vec<String>) -> HashMap<&str, String> {
    // 使用 `Command` 运行 Python 脚本
    println!("{:?}", script_path);
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
pub async fn run_python_plugin(
    interpreter: Option<String>,
    script_path: String,
    request: Value,
    timeout_ms: Option<u64>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_python_plugin_blocking(interpreter, script_path, request, timeout_ms)
    })
    .await
    .map_err(|error| format!("Python 任务失败：{error}"))?
}

fn run_python_plugin_blocking(
    interpreter: Option<String>,
    script_path: String,
    request: Value,
    timeout_ms: Option<u64>,
) -> Result<Value, String> {
    use std::io::Read;
    use std::sync::mpsc;

    let executable = interpreter
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "python.exe".into()
            } else {
                "python3".into()
            }
        });
    let body = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);
    let mut child = command
        .arg(script_path)
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 Python，请检查解释器是否已安装或配置正确：{error}"))?;
    let mut stdin = child.stdin.take().ok_or("无法连接 Python 输入")?;
    let mut stdout = child.stdout.take().ok_or("无法连接 Python 输出")?;
    let mut stderr = child.stderr.take().ok_or("无法连接 Python 错误输出")?;
    let (input_sender, input_receiver) = mpsc::channel();
    let (output_sender, output_receiver) = mpsc::channel();
    let (error_sender, error_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = input_sender.send(stdin.write_all(&body));
    });
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.read_to_end(&mut bytes).map(|_| bytes);
        let _ = output_sender.send(result);
    });
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stderr.read_to_end(&mut bytes).map(|_| bytes);
        let _ = error_sender.send(result);
    });
    let limit = Duration::from_millis(timeout_ms.unwrap_or(30_000));
    let started = Instant::now();
    let mut input = None;
    let mut output = None;
    let mut errors = None;
    let status = loop {
        if input.is_none() {
            input = input_receiver.try_recv().ok();
        }
        if output.is_none() {
            output = output_receiver.try_recv().ok();
        }
        if errors.is_none() {
            errors = error_receiver.try_recv().ok();
        }
        match child.try_wait() {
            Ok(Some(status)) if input.is_some() && output.is_some() && errors.is_some() => {
                break status
            }
            Ok(_) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("读取 Python 进程状态失败：{error}"));
            }
        }
        if started.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Python 执行超时，已停止进程".to_string());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = output.unwrap().map_err(|error| error.to_string())?;
    let errors = errors.unwrap().map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!(
            "Python 退出状态 {}：{}",
            status,
            String::from_utf8_lossy(&errors)
        ));
    }
    input
        .unwrap()
        .map_err(|error| format!("传递 Python 参数失败：{error}"))?;
    let stdout = String::from_utf8_lossy(&output);
    serde_json::from_str(stdout.trim()).map_err(|e| format!("invalid python response: {e}"))
}

#[cfg(test)]
mod python_plugin_tests {
    use super::*;

    struct ScriptFile(std::path::PathBuf);

    impl ScriptFile {
        fn new(source: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "lark-python-test-{}-{nonce}.py",
                std::process::id()
            ));
            fs::write(&path, source).unwrap();
            Self(path)
        }

        fn run(&self, request: Value, timeout: u64) -> Result<Value, String> {
            run_python_plugin_blocking(
                None,
                self.0.to_string_lossy().to_string(),
                request,
                Some(timeout),
            )
        }
    }

    impl Drop for ScriptFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn plugin_python_large_streams_and_unicode() {
        let script = ScriptFile::new("import json, sys\nrequest = json.load(sys.stdin)\nsys.stderr.write('diagnostic' * 100000)\nprint(json.dumps({'ok': True, 'result': request['text']}))\n");
        let text = "中文输入".repeat(100000);
        let result = script
            .run(serde_json::json!({"text": text}), 10000)
            .unwrap();
        assert_eq!(result["result"], text);
    }

    #[test]
    fn plugin_python_timeout_and_missing_interpreter() {
        let script = ScriptFile::new("import time\ntime.sleep(5)\n");
        let error = script.run(serde_json::json!({}), 100).unwrap_err();
        assert!(error.contains("超时"));
        let missing = script
            .0
            .with_extension("missing-executable")
            .to_string_lossy()
            .to_string();
        let error = run_python_plugin_blocking(
            Some(missing),
            script.0.to_string_lossy().to_string(),
            serde_json::json!({}),
            Some(100),
        )
        .unwrap_err();
        assert!(error.contains("无法启动 Python"));
    }

    #[test]
    fn plugin_python_invalid_output_and_nonzero_exit() {
        let script = ScriptFile::new("print('not json')\n");
        assert!(script
            .run(serde_json::json!({}), 5000)
            .unwrap_err()
            .contains("invalid python response"));
        let script =
            ScriptFile::new("import sys\nsys.stderr.write('intentional error')\nsys.exit(2)\n");
        assert!(script
            .run(serde_json::json!({}), 5000)
            .unwrap_err()
            .contains("intentional error"));
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
    // Some entries are indexed as "apps" for launcher purposes even though
    // they are files opened by a registered default application. Delegate
    // those paths to the file association handler instead of CreateProcessW.
    let extension = Path::new(app_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    if matches!(extension.as_deref(), Some("rdp" | "url")) {
        open_file(app_path);
        return;
    }

    let current_dir = Path::new(app_path).parent().unwrap();
    let program = app_path.split("\\").last().expect("aa.exe");
    println!("打开app:{:?}", app_path);
    if crate::api::explorer::is_process_running(program) {
        // 激活窗口
        println!("Process {} is already running.", app_name);
        crate::api::explorer::find_windows_with_partial_title(app_name);
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
pub fn clipboard_control(
    text: &str,
    control: &str,
    paste: bool,
    data_type: &str,
) -> Result<String, String> {
    // 按字符截取预览，避免中文等多字节字符在 byte index 处切片导致 panic。
    let preview: String = text.chars().take(100).collect();
    println!("[{}]剪贴板控制：{:?}", data_type, preview);
    if control == "write" {
        if data_type == "file" {
            let files: Vec<(String, String)> = serde_json::from_str(text)
                .map_err(|error| format!("文件剪贴板数据格式错误: {error}"))?;
            ClipboardOperator::set_file(&files).map_err(|error| error.to_string())?;
            if paste {
                ClipboardOperator::paste_text().map_err(|error| error.to_string())?;
            }
            Ok("写入文件剪贴板成功".to_string())
        } else if data_type == "image" {
            let img = ImageDataDB {
                base64: text.to_string(),
                ..Default::default()
            };
            ClipboardOperator::set_image(img).map_err(|error| error.to_string())?;
            if paste {
                ClipboardOperator::paste_text().map_err(|error| error.to_string())?;
            }
            println!("写入剪贴板成功");
            Ok("写入剪贴板成功".to_string())
        } else {
            if paste {
                ClipboardOperator::set_text_for_paste(text)
            } else {
                ClipboardOperator::set_text(text)
            }
            .map_err(|error| error.to_string())?;
            if paste {
                ClipboardOperator::paste_text().map_err(|error| error.to_string())?;
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
    let output = run_python_script(
        "D:/Project/Lark/src-tauri/target/debug/config/lark/data/plugins/PrettyPostman/str2json.py",
        vec![],
    );
    print!("{:?}", output);
}
