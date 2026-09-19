#![cfg(windows)]

//! 注入脚本 + 处理 WebView2 的 web message，把被拖入文件的绝对路径发给前端。

use serde::{Deserialize, Serialize};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, Runtime};
use webview2_com::{
    ExecuteScriptCompletedHandler,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs2,
    },
    WebMessageReceivedEventHandler,
};
use windows::core::{Interface, PCWSTR, PWSTR};

/// 与注入脚本约定的消息标记，用于在 WebMessageReceived 里过滤出我们自己的消息。
const MESSAGE: &str = "__LARK_WIN_FILE_DROP__";

/// 注入到页面的脚本：
/// - `dragover` 里对文件拖放调用 preventDefault，否则浏览器不会派发 `drop`（自包含，不依赖页面代码）；
/// - `drop` 时把 File 对象交给宿主，由宿主解析出绝对路径。
const INJECT_SCRIPT: &str = r#"
    (() => {
        const MESSAGE = "__LARK_WIN_FILE_DROP__";
        const isFileDrag = (e) =>
            !!e.dataTransfer && Array.from(e.dataTransfer.types || []).includes("Files");
        const onDragOver = (e) => { if (isFileDrag(e)) e.preventDefault(); };
        const onDrop = (e) => {
            if (isFileDrag(e) && e.dataTransfer.files && e.dataTransfer.files.length) {
                e.preventDefault();
                window.chrome.webview.postMessageWithAdditionalObjects(MESSAGE, e.dataTransfer.files);
            }
        };
        document.removeEventListener("dragover", onDragOver);
        document.addEventListener("dragover", onDragOver);
        document.removeEventListener("drop", onDrop);
        document.addEventListener("drop", onDrop);
    })();
"#;

/// 与 Tauri/wry 原生拖放事件一致的 payload 形状，前端可直接复用。
#[derive(Serialize, Deserialize, Clone)]
struct DropEvent {
    paths: Vec<PathBuf>,
    position: PhysicalPosition<f64>,
}

fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    string
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 为某个 webview 注册拖放通道。
pub fn register<R: Runtime>(app: &AppHandle<R>, label: &str, webview2: &ICoreWebView2) {
    let script = encode_wide(INJECT_SCRIPT);
    let _ = unsafe {
        webview2.ExecuteScript(
            PCWSTR::from_raw(script.as_ptr()),
            &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
        )
    };

    // add_WebMessageReceived 是追加注册，不会顶掉 Tauri 自己的 IPC 处理。
    let mut token = 0;
    let app = app.clone();
    let label = label.to_string();
    let _ = unsafe {
        webview2.add_WebMessageReceived(
            &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };

                let mut message = PWSTR::null();
                args.TryGetWebMessageAsString(&mut message)?;
                if message.to_string()? != MESSAGE {
                    return Ok(());
                }

                let Ok(args2) = args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>() else {
                    return Ok(());
                };
                let Ok(objects) = args2.AdditionalObjects() else {
                    return Ok(());
                };

                let mut count = 0;
                objects.Count(&mut count)?;
                let mut paths = Vec::new();
                for index in 0..count {
                    let Ok(value) = objects.GetValueAtIndex(index) else {
                        continue;
                    };
                    let Ok(file) = value.cast::<ICoreWebView2File>() else {
                        continue;
                    };
                    let mut path_ptr = PWSTR::null();
                    if file.Path(&mut path_ptr).is_ok() {
                        if let Ok(path) = path_ptr.to_string() {
                            paths.push(PathBuf::from(path));
                        }
                    }
                }
                if paths.is_empty() {
                    return Ok(());
                }

                if let Some(window) = app.get_webview_window(&label) {
                    let payload = DropEvent {
                        paths,
                        // 前端用不到坐标；要拿窗口内相对位置还得额外导入 WindowsExt trait。
                        // 这里给 0，保持 payload 形状与 Tauri 原生事件一致。
                        position: PhysicalPosition::new(0.0, 0.0),
                    };
                    let _ = window.emit_to(
                        EventTarget::WebviewWindow {
                            label: label.clone(),
                        },
                        "tauri://drag-drop",
                        payload,
                    );
                }

                Ok(())
            })),
            &mut token,
        )
    };
}
