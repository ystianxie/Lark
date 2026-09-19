//! Windows Explorer 前台路径监听器。
//!
//! 第一版采用低频轮询：获取前台窗口，再通过 `IShellWindows` 找到相同 HWND
//! 的 Explorer 窗口并读取 `LocationURL`。模块不会自动启动；调用
//! [`start_test_listener`] 即可手工验证。

#[cfg(target_os = "windows")]
mod platform {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use windows::core::{Interface, VARIANT};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_ALL,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellWindows, IWebBrowserApp, ShellWindows};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    use crate::api::dialog_probe::{probe_foreground_dialog, DialogConfidence};

    /// 可停止测试监听线程的句柄。
    pub struct ExplorerListenerHandle {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl ExplorerListenerHandle {
        /// 请求停止监听并等待线程退出。
        pub fn stop(mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    impl Drop for ExplorerListenerHandle {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    struct ComApartment;

    impl ComApartment {
        fn initialize() -> Result<Self, String> {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                    .ok()
                    .map_err(|error| format!("初始化 COM 失败: {error}"))?;
            }
            Ok(Self)
        }
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    /// 单次读取当前前台 Explorer 的文件系统路径。
    ///
    /// 当前前台窗口不是 Explorer，或 Explorer 位于“此电脑”等非文件系统位置时，
    /// 返回 `Ok(None)`。
    pub fn foreground_explorer_path() -> Result<Option<PathBuf>, String> {
        let _com = ComApartment::initialize()?;
        foreground_explorer_path_in_current_apartment()
    }

    /// 启动后台轮询；当前台 Explorer 路径发生变化时调用回调。
    ///
    /// 回调只在路径变化时执行；切到非 Explorer 窗口不会清空最后路径。
    pub fn start_listener<F>(on_path_changed: F) -> ExplorerListenerHandle
    where
        F: Fn(PathBuf) + Send + 'static,
    {
        start_listener_with_dialog(on_path_changed, |_| {})
    }

    pub fn start_listener_with_dialog<F, D>(
        on_path_changed: F,
        on_dialog_changed: D,
    ) -> ExplorerListenerHandle
    where
        F: Fn(PathBuf) + Send + 'static,
        D: Fn(crate::api::dialog_probe::DialogProbeResult) + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);

        let thread = thread::spawn(move || {
            if let Err(error) = run_listener(
                thread_stop,
                Duration::from_millis(500),
                on_path_changed,
                on_dialog_changed,
            ) {
                eprintln!("[ExplorerListener] {error}");
            }
        });

        ExplorerListenerHandle {
            stop,
            thread: Some(thread),
        }
    }

    /// 测试入口：启动后台轮询；当前台 Explorer 路径发生变化时打印路径。
    ///
    /// 此函数不会自动接入 Tauri 生命周期。调用者应保存返回值，并在不再需要时
    /// 调用 `handle.stop()`。如果直接丢弃返回值，监听线程会收到停止请求。
    pub fn start_test_listener() -> ExplorerListenerHandle {
        start_listener(|path| {
            println!("[ExplorerListener] 前台 Explorer: {}", path.display());
        })
    }

    fn run_listener<F>(
        stop: Arc<AtomicBool>,
        interval: Duration,
        on_path_changed: F,
        on_dialog_changed: impl Fn(crate::api::dialog_probe::DialogProbeResult),
    ) -> Result<(), String>
    where
        F: Fn(PathBuf),
    {
        let _com = ComApartment::initialize()?;
        let mut last_path: Option<PathBuf> = None;
        let mut last_path_state: Option<Option<PathBuf>> = None;
        let mut last_dialog_signature: Option<(isize, DialogConfidence, u8)> = None;
        // 只有从资源管理器直接切换到文件对话框，才执行自动跳转。
        let mut previous_foreground_was_explorer = false;

        println!(
            "[ExplorerListener] 已启动，轮询间隔 {} ms",
            interval.as_millis()
        );

        while !stop.load(Ordering::Relaxed) {
            let current_foreground_is_explorer = match foreground_explorer_path_in_current_apartment(
            ) {
                Ok(path_state) => {
                    if last_path_state.as_ref() != Some(&path_state) {
                        match &path_state {
                                Some(path) => println!(
                                    "[ExplorerListener] 当前识别到的资源管理器路径: {}",
                                    path.display()
                                ),
                                None => println!(
                                    "[ExplorerListener] 当前未识别到资源管理器路径（前台窗口不是文件夹或路径不可用）"
                                ),
                            }
                        last_path_state = Some(path_state.clone());
                    }

                    let is_explorer = path_state.is_some();
                    if let Some(path) = path_state {
                        if last_path.as_ref() != Some(&path) {
                            last_path = Some(path.clone());
                            on_path_changed(path);
                        }
                    }
                    is_explorer
                }
                Err(error) => {
                    eprintln!("[ExplorerListener] 查询失败: {error}");
                    previous_foreground_was_explorer
                }
            };

            match probe_foreground_dialog() {
                Ok(result) => {
                    let signature = (result.hwnd, result.confidence, result.score);
                    if last_dialog_signature != Some(signature) {
                        println!(
                            "[ExplorerListener] 文件选择框识别: {:?}, hwnd=0x{:X}, title={:?}, class={:?}, score={}, evidence={}",
                            result.confidence,
                            result.hwnd,
                            result.title,
                            result.window_class,
                            result.score,
                            result.evidence.join("；")
                        );
                        last_dialog_signature = Some(signature);

                        if previous_foreground_was_explorer
                            && !current_foreground_is_explorer
                            && result.confidence == DialogConfidence::Confirmed
                        {
                            println!(
                                "[ExplorerListener] 检测到资源管理器 -> 文件选择框，触发自动跳转"
                            );
                            on_dialog_changed(result);
                        }
                    }
                }
                Err(error) => eprintln!("[ExplorerListener] 文件选择框探测失败: {error}"),
            }

            previous_foreground_was_explorer = current_foreground_is_explorer;
            thread::sleep(interval);
        }

        println!("[ExplorerListener] 已停止");
        Ok(())
    }

    fn foreground_explorer_path_in_current_apartment() -> Result<Option<PathBuf>, String> {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.0.is_null() {
            return Ok(None);
        }

        let shell_windows: IShellWindows = unsafe {
            CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)
                .map_err(|error| format!("创建 ShellWindows 失败: {error}"))?
        };

        let count = unsafe { shell_windows.Count() }
            .map_err(|error| format!("读取 ShellWindows 数量失败: {error}"))?;

        for index in 0..count {
            let item: IDispatch = match unsafe { shell_windows.Item(&VARIANT::from(index)) } {
                Ok(item) => item,
                Err(_) => continue,
            };
            let browser: IWebBrowserApp = match item.cast() {
                Ok(browser) => browser,
                Err(_) => continue,
            };
            let explorer_hwnd = match unsafe { browser.HWND() } {
                Ok(hwnd) => hwnd,
                Err(_) => continue,
            };

            if explorer_hwnd.0 != foreground.0 as isize {
                continue;
            }

            let location_url = unsafe { browser.LocationURL() }
                .map_err(|error| format!("读取 Explorer LocationURL 失败: {error}"))?
                .to_string();

            return Ok(file_url_to_path(&location_url));
        }

        Ok(None)
    }

    fn file_url_to_path(url: &str) -> Option<PathBuf> {
        let encoded = url.strip_prefix("file://")?;
        let decoded = percent_decode(encoded)?;

        let path = if decoded.starts_with('/') && decoded.as_bytes().get(2).copied() == Some(b':') {
            // file:///C:/dir -> C:/dir
            decoded[1..].to_string()
        } else if !decoded.starts_with('/') {
            // file://server/share -> \\server\share
            format!(r"\\{}", decoded)
        } else {
            return None;
        };

        let path = PathBuf::from(path.replace('/', r"\"));
        path.is_dir().then_some(path)
    }

    fn percent_decode(value: &str) -> Option<String> {
        let bytes = value.as_bytes();
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut index = 0;

        while index < bytes.len() {
            if bytes[index] == b'%' {
                let high = *bytes.get(index + 1)?;
                let low = *bytes.get(index + 2)?;
                decoded.push((hex_value(high)? << 4) | hex_value(low)?);
                index += 3;
            } else {
                decoded.push(bytes[index]);
                index += 1;
            }
        }

        String::from_utf8(decoded).ok()
    }

    fn hex_value(value: u8) -> Option<u8> {
        match value {
            b'0'..=b'9' => Some(value - b'0'),
            b'a'..=b'f' => Some(value - b'a' + 10),
            b'A'..=b'F' => Some(value - b'A' + 10),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::percent_decode;

        #[test]
        fn decodes_utf8_file_url_component() {
            assert_eq!(
                percent_decode("C:/Users/test/%E4%B8%AD%E6%96%87"),
                Some("C:/Users/test/中文".to_string())
            );
        }

        #[test]
        fn rejects_incomplete_percent_encoding() {
            assert_eq!(percent_decode("C:/broken%2"), None);
        }
    }
}

#[cfg(target_os = "windows")]
pub use platform::{
    foreground_explorer_path, start_listener, start_listener_with_dialog, start_test_listener,
    ExplorerListenerHandle,
};

#[cfg(not(target_os = "windows"))]
pub fn foreground_explorer_path() -> Result<Option<std::path::PathBuf>, String> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
pub fn start_listener_with_dialog<F, D>(
    _on_path_changed: F,
    _on_dialog_changed: D,
) -> explorer_listener_handle_placeholder::ExplorerListenerHandle
where
    F: Fn(std::path::PathBuf) + Send + 'static,
    D: Send + 'static,
{
    explorer_listener_handle_placeholder::ExplorerListenerHandle
}

#[cfg(not(target_os = "windows"))]
mod explorer_listener_handle_placeholder {
    pub struct ExplorerListenerHandle;
}

#[test]
fn test() {
    match foreground_explorer_path() {
        Ok(Some(path)) => {
            println!("当前 Explorer 路径：{}", path.display());
        }
        Ok(None) => {
            println!("当前前台窗口不是 Explorer，或者不是文件系统目录");
        }
        Err(error) => {
            eprintln!("获取失败：{error}");
        }
    }
}
