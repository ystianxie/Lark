//! 取回被拖入窗口的文件的真实路径（仅 Windows 有效，其他平台为空实现）。
//!
//! 背景：WebView2 出于安全策略不把本地文件路径暴露给页面 —— HTML5 的 `dataTransfer`
//! 里只有 `Files`，`text/uri-list` 是空串；而 Tauri/wry 自己的方案（`dragDropEnabled: true`）
//! 会替换掉 WebView2 的拖放处理，代价是 HTML5 拖放整体失效。
//!
//! 这里走第三条路：注入脚本把 `drop` 到的 `File` 通过
//! `chrome.webview.postMessageWithAdditionalObjects` 交给宿主，宿主再用
//! `ICoreWebView2WebMessageReceivedEventArgs2::AdditionalObjects` → `ICoreWebView2File::Path`
//! 拿到绝对路径，最后以 `tauri://drag-drop` 事件（payload 形如 `{ paths, position }`）发回前端。
//! 全程不替换 WebView2 的拖放处理，因此 HTML5 拖放仍然可用。

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
    Manager
};

#[cfg(windows)]
mod desktop;

/// 初始化插件。非 Windows 平台直接返回一个不做任何事的插件。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = Builder::new("win-file-drop");

    #[cfg(windows)]
    let builder = builder.on_webview_ready(|webview| {
        let app = webview.app_handle().clone();
        let label = webview.label().to_string();
        let _ = webview.with_webview(move |tauri_webview| {
            // SAFETY: with_webview 保证在 WebView2 的 UI 线程上回调。
            if let Ok(webview2) = unsafe { tauri_webview.controller().CoreWebView2() } {
                desktop::register(&app, &label, &webview2);
            }
        });
    });

    builder.build()
}
