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

/// 平台默认解释器命令。未配置时使用，保持与历史行为一致。
fn platform_default_python() -> String {
    if cfg!(target_os = "windows") {
        "python.exe".to_string()
    } else {
        "python3".to_string()
    }
}

/// 解释器解析的唯一入口：配置值（非空）优先，否则回落平台默认命令。
/// 纯函数，不查路径是否存在——路径真实性交给 `probe_python_interpreter`。
fn resolve_python_interpreter(configured: Option<&str>) -> String {
    configured
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .unwrap_or_else(platform_default_python)
}

/// 从宿主配置读取用户在「应用设置」里配置的解释器。
/// 每次调用都重新读配置：插件实时执行需要「改完配置立即生效」，不能缓存快照。
fn configured_python_interpreter() -> String {
    let configured = crate::config::Config::read_local_config()
        .ok()
        .and_then(|config| config.base.python_interpreter);
    resolve_python_interpreter(configured.as_deref())
}

#[tauri::command(rename_all = "camelCase")]
pub fn run_python_script(script_path: &str, params: Vec<String>) -> HashMap<&str, String> {
    // 使用 `Command` 运行 Python 脚本。解释器与插件执行共用同一份设置。
    println!("{:?}", script_path);
    let executable = configured_python_interpreter();
    let mut command = Command::new(&executable);
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);
    let output = command
        .arg(script_path)
        .args(params)
        .env("PYTHONIOENCODING", "utf-8")
        .output();
    let mut result = HashMap::new();
    match output {
        Ok(output) if output.status.success() => {
            // 将输出转换为字符串并打印
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("Script output:\n{}", stdout);
            result.insert("success", "true".to_string());
            result.insert("data", stdout.to_string());
        }
        Ok(output) => {
            // 如果脚本执行失败，打印错误信息
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Script output:\n{}", stderr);
            result.insert("success", "false".to_string());
            result.insert("data", stderr.to_string());
        }
        Err(error) => {
            // 解释器不存在时不能 panic：该命令同步执行，panic 会跨过 IPC 边界终止进程。
            // 继续沿用 success/data 契约，调用方（App.jsx 的外部索引脚本）只认这两个键。
            let message = format!("无法启动 Python（{executable}）：{error}");
            println!("{message}");
            result.insert("success", "false".to_string());
            result.insert("data", message);
        }
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
pub async fn run_python_plugin(
    script_path: String,
    request: Value,
    timeout_ms: Option<u64>,
) -> Result<Value, String> {
    // 解释器只由宿主设置决定（manifest 里的 runtime 字段已废弃）。
    // 读配置放在阻塞线程里，避免拖慢 async 线程。
    tauri::async_runtime::spawn_blocking(move || {
        run_python_plugin_blocking(
            configured_python_interpreter(),
            script_path,
            request,
            timeout_ms,
        )
    })
    .await
    .map_err(|error| format!("Python 任务失败：{error}"))?
}

fn run_python_plugin_blocking(
    executable: String,
    script_path: String,
    request: Value,
    timeout_ms: Option<u64>,
) -> Result<Value, String> {
    use std::io::Read;
    use std::sync::mpsc;

    let body = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
    let mut command = Command::new(&executable);
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
    let mut response: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("invalid python response: {e}"))?;
    let stderr = String::from_utf8_lossy(&errors);
    if !stderr.is_empty() {
        let object = response
            .as_object_mut()
            .ok_or("invalid python response: expected a JSON object")?;
        // stdout 仍然只承载 Python JSON 协议；业务代码的 print 被包装层重定向到
        // stderr。将它放进宿主保留字段交给 WebView 输出，前端会在把响应交还插件前
        // 删除该字段，因此不改变 runPython 的公开返回契约。
        object.insert(
            "__larkPythonStderr".to_string(),
            Value::String(stderr.into_owned()),
        );
    }
    Ok(response)
}

/// 从解释器所在目录向上逐级查找 `pyvenv.cfg`，判断它是否属于某个虚拟环境。
fn python_environment_kind(executable: &str) -> &'static str {
    let path = Path::new(executable);
    // 只有绝对路径才向上查找：裸命令名（"python.exe"）的 parent 是空路径，
    // 照直查找会把当前工作目录里的 pyvenv.cfg 误判成虚拟环境。
    if !path.is_absolute() {
        return "system";
    }
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join("pyvenv.cfg").is_file() {
            return "venv";
        }
        current = dir.parent();
    }
    "system"
}

/// Microsoft Store 的 App Execution Alias 目录。只用于在探测失败时给出更贴近
/// 用户操作的提示，不能单凭路径下结论——真装了 Store 版 Python 也能正常探测。
fn is_windows_store_alias(executable: &str) -> bool {
    executable
        .replace('\\', "/")
        .to_ascii_lowercase()
        .contains("windowsapps")
}

/// 执行 `<executable> --version` 并返回版本号文本。
/// Python 3 把版本写到 stdout、2.x 写到 stderr，两路都要取。
fn probe_python_version(executable: &str, timeout: Duration) -> Result<String, String> {
    let mut command = Command::new(executable);
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);
    let mut child = command
        .arg("--version")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动解释器：{error}"))?;
    // --version 只有一行输出，管道不会写满，因此不需要 run_python_plugin_blocking 那套
    // 多线程读管道；这里只需一个超时兜底，防止解释器卡住不退出。
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("探测超时，已停止进程".to_string());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("读取解释器进程状态失败：{error}"));
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("读取解释器输出失败：{error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(if detail.is_empty() {
            format!("解释器返回非零退出码：{}", output.status)
        } else {
            detail
        })
    }
}

fn probe_python_interpreter_blocking(configured: Option<String>) -> Value {
    let configured = configured
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());
    let resolved = resolve_python_interpreter(configured.as_deref());
    let mut kind = python_environment_kind(&resolved);
    let (ok, version, error) = match probe_python_version(&resolved, Duration::from_secs(5)) {
        Ok(version) => (true, Some(version), None),
        Err(error) => {
            if is_windows_store_alias(&resolved) {
                kind = "store-alias";
            }
            (false, None, Some(error))
        }
    };
    serde_json::json!({
        "ok": ok,
        "configured": configured,
        "resolved": resolved,
        "version": version,
        "kind": kind,
        "error": error,
    })
}

/// 探测解释器是否可用，供「应用设置 - Python 环境」做即时反馈。
/// 未配置（path 为 None）时探测平台默认命令，便于界面显示「当前默认」。
#[tauri::command(rename_all = "camelCase")]
pub async fn probe_python_interpreter(path: Option<String>) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || probe_python_interpreter_blocking(path))
        .await
        .map_err(|error| format!("Python 探测失败：{error}"))
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
                resolve_python_interpreter(None),
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
        assert!(result["__larkPythonStderr"]
            .as_str()
            .unwrap_or_default()
            .starts_with("diagnostic"));
    }

    #[test]
    fn plugin_python_exposes_stderr_for_frontend_logging() {
        let script = ScriptFile::new(
            "import json, sys\nsys.stderr.write('print from plugin\\n')\nprint(json.dumps({'ok': True, 'result': 42}))\n",
        );
        let result = script.run(serde_json::json!({}), 5000).unwrap();
        assert_eq!(result["result"], 42);
        assert_eq!(
            result["__larkPythonStderr"]
                .as_str()
                .unwrap_or_default()
                .trim_end(),
            "print from plugin"
        );
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
            missing,
            script.0.to_string_lossy().to_string(),
            serde_json::json!({}),
            Some(100),
        )
        .unwrap_err();
        assert!(error.contains("无法启动 Python"));
    }

    #[test]
    fn interpreter_resolution_prefers_configured_path() {
        assert_eq!(
            resolve_python_interpreter(Some(r"D:\envs\x\Scripts\python.exe")),
            r"D:\envs\x\Scripts\python.exe"
        );
        let default = resolve_python_interpreter(None);
        assert!(default == "python.exe" || default == "python3");
        // 空白配置等同于未配置：否则会把空串当成可执行文件名交给 Command::new。
        assert_eq!(resolve_python_interpreter(Some("   ")), default);
        assert_eq!(resolve_python_interpreter(Some("")), default);
    }

    #[test]
    fn python_environment_kind_detects_venv_and_ignores_command_names() {
        // 裸命令名不能触发向上查找，否则会把工作目录里的 pyvenv.cfg 误判成虚拟环境。
        assert_eq!(python_environment_kind("python.exe"), "system");
        let root = std::env::temp_dir().join(format!(
            "lark-venv-probe-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let scripts = root.join("Scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(root.join("pyvenv.cfg"), "home = C:\\Python312\n").unwrap();
        let interpreter = scripts.join("python.exe").to_string_lossy().to_string();
        assert_eq!(python_environment_kind(&interpreter), "venv");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn probe_reports_missing_interpreter_without_panicking() {
        let missing = std::env::temp_dir()
            .join(format!("lark-missing-python-{}", std::process::id()))
            .to_string_lossy()
            .to_string();
        let probe = probe_python_interpreter_blocking(Some(missing));
        assert_eq!(probe["ok"], serde_json::json!(false));
        assert!(probe["error"]
            .as_str()
            .unwrap_or_default()
            .contains("无法启动"));
        assert!(probe["resolved"].is_string());
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
    if app_path
        .to_ascii_lowercase()
        .starts_with("shell:appsfolder\\")
    {
        open_file(app_path);
        return;
    }

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
        // 目录、非 UTF-8 文本、被占用等情况都会让 read_to_string 失败。
        // 这里必须转成 Err 返回：命令是同步执行的，panic 会跨过 IPC 回调边界直接终止进程。
        let content = fs::read_to_string(file_path).map_err(|error| error.to_string())?;
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
