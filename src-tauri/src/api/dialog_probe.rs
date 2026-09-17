//! Windows 文件对话框识别探针。
//!
//! 第一版只读取窗口结构，不执行跳转，也不修改目标窗口。调用
//! [`probe_foreground_dialog`] 可单次检查，调用 [`start_test_dialog_probe`]
//! 可启动轮询测试。

#[cfg(target_os = "windows")]
mod platform {
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, GA_ROOT,
    };

    /// 对前台窗口是文件对话框的置信度。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DialogConfidence {
        /// 窗口结构符合标准 Windows 文件对话框。
        Confirmed,
        /// 看起来像对话框，但证据不足；正式功能不应在此状态吞掉快捷键。
        Probable,
        /// 当前窗口不是已识别的文件对话框。
        NotDialog,
    }

    /// 单次探测结果，保留判定证据，方便后续适配不同应用。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DialogProbeResult {
        pub hwnd: isize,
        pub process_id: u32,
        pub title: String,
        pub window_class: String,
        pub confidence: DialogConfidence,
        pub score: u8,
        pub evidence: Vec<String>,
        pub child_classes: Vec<String>,
    }

    /// 可停止的测试探针线程句柄。
    pub struct DialogProbeHandle {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl DialogProbeHandle {
        pub fn stop(mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    impl Drop for DialogProbeHandle {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    #[derive(Default)]
    struct ChildWindowSummary {
        classes: BTreeSet<String>,
        visible_button_count: usize,
        visible_edit_count: usize,
    }

    /// 单次识别当前前台窗口。
    ///
    /// 该函数只读取窗口元数据和子控件类名，不会操作对话框。
    pub fn probe_foreground_dialog() -> Result<DialogProbeResult, String> {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.0.is_null() {
            return Err("当前没有前台窗口".to_string());
        }

        let root = unsafe { GetAncestor(foreground, GA_ROOT) };
        let hwnd = if root.0.is_null() { foreground } else { root };
        probe_window(hwnd)
    }

    /// 测试入口：每 500ms 探测一次，当前台窗口或判定结果变化时打印。
    pub fn start_test_dialog_probe() -> DialogProbeHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || run_probe_loop(thread_stop));

        DialogProbeHandle {
            stop,
            thread: Some(thread),
        }
    }

    fn run_probe_loop(stop: Arc<AtomicBool>) {
        let interval = Duration::from_millis(500);
        let mut last_signature: Option<(isize, DialogConfidence, u8)> = None;

        println!("[DialogProbe] 已启动，轮询间隔 {} ms", interval.as_millis());

        while !stop.load(Ordering::Relaxed) {
            match probe_foreground_dialog() {
                Ok(result) => {
                    let signature = (result.hwnd, result.confidence, result.score);
                    if last_signature != Some(signature) {
                        print_result(&result);
                        last_signature = Some(signature);
                    }
                }
                Err(error) => eprintln!("[DialogProbe] 探测失败: {error}"),
            }
            thread::sleep(interval);
        }

        println!("[DialogProbe] 已停止");
    }

    fn print_result(result: &DialogProbeResult) {
        println!(
            "[DialogProbe] {:?} score={} hwnd=0x{:X} pid={} class={:?} title={:?}",
            result.confidence,
            result.score,
            result.hwnd,
            result.process_id,
            result.window_class,
            result.title
        );
        println!("[DialogProbe] 证据: {}", result.evidence.join("；"));
        println!(
            "[DialogProbe] 子控件类: {}",
            result.child_classes.join(", ")
        );
    }

    fn probe_window(hwnd: HWND) -> Result<DialogProbeResult, String> {
        let window_class = window_class(hwnd);
        let title = window_text(hwnd);
        let process_id = window_process_id(hwnd);
        let children = enumerate_child_windows(hwnd)?;

        let mut score = 0u8;
        let mut evidence = Vec::new();

        let is_standard_dialog = window_class == "#32770";
        if is_standard_dialog {
            score += 2;
            evidence.push("顶层窗口类为 #32770".to_string());
        }

        let has_direct_ui = children.classes.contains("DirectUIHWND");
        if has_direct_ui {
            score += 2;
            evidence.push("包含 DirectUIHWND Shell 视图".to_string());
        }

        let has_shell_container = children.classes.contains("DUIViewWndClassName")
            || children.classes.contains("SHELLDLL_DefView")
            || children.classes.contains("SysListView32");
        if has_shell_container {
            score += 2;
            evidence.push("包含文件列表/Shell 视图控件".to_string());
        }

        let has_navigation_controls = children.classes.contains("ComboBoxEx32")
            || children.classes.contains("Breadcrumb Parent")
            || children.classes.contains("ToolbarWindow32");
        if has_navigation_controls {
            score += 1;
            evidence.push("包含地址导航控件".to_string());
        }

        if children.visible_edit_count > 0 {
            score += 1;
            evidence.push(format!(
                "包含 {} 个可见输入控件",
                children.visible_edit_count
            ));
        }

        if children.visible_button_count >= 2 {
            score += 1;
            evidence.push(format!("包含 {} 个可见按钮", children.visible_button_count));
        }

        // #32770 很常见，必须同时存在文件视图证据才能确认为文件对话框。
        let confidence = if is_standard_dialog
            && (has_direct_ui || has_shell_container)
            && children.visible_button_count > 0
        {
            DialogConfidence::Confirmed
        } else if is_standard_dialog && score >= 3 {
            DialogConfidence::Probable
        } else {
            DialogConfidence::NotDialog
        };

        if evidence.is_empty() {
            evidence.push("未发现标准文件对话框特征".to_string());
        }

        Ok(DialogProbeResult {
            hwnd: hwnd.0 as isize,
            process_id,
            title,
            window_class,
            confidence,
            score,
            evidence,
            child_classes: children.classes.into_iter().collect(),
        })
    }

    fn enumerate_child_windows(hwnd: HWND) -> Result<ChildWindowSummary, String> {
        let mut summary = ChildWindowSummary::default();
        let parameter = LPARAM((&mut summary as *mut ChildWindowSummary) as isize);

        let result = unsafe { EnumChildWindows(hwnd, Some(enum_child_proc), parameter) };
        if !result.as_bool() {
            // EnumChildWindows 返回 FALSE 也可能只是枚举自然结束；只要已采集到结构就可使用。
            // 此回调始终返回 TRUE，所以空结果同样是合法的“没有子窗口”。
        }

        Ok(summary)
    }

    unsafe extern "system" fn enum_child_proc(hwnd: HWND, parameter: LPARAM) -> BOOL {
        let summary = &mut *(parameter.0 as *mut ChildWindowSummary);
        let class_name = window_class(hwnd);
        let visible = IsWindowVisible(hwnd).as_bool();

        if !class_name.is_empty() {
            if visible && class_name == "Button" {
                summary.visible_button_count += 1;
            }
            if visible && (class_name == "Edit" || class_name == "RichEdit20W") {
                summary.visible_edit_count += 1;
            }
            summary.classes.insert(class_name);
        }

        BOOL(1)
    }

    fn window_class(hwnd: HWND) -> String {
        let mut buffer = [0u16; 256];
        let length = unsafe { GetClassNameW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }

    fn window_text(hwnd: HWND) -> String {
        let mut buffer = [0u16; 512];
        let length = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }

    fn window_process_id(hwnd: HWND) -> u32 {
        let mut process_id = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
        process_id
    }
}

#[cfg(target_os = "windows")]
pub use platform::{
    probe_foreground_dialog, start_test_dialog_probe, DialogConfidence, DialogProbeHandle,
    DialogProbeResult,
};
#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogConfidence {
    Confirmed,
    Probable,
    NotDialog,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogProbeResult {
    pub hwnd: isize,
    pub process_id: u32,
    pub title: String,
    pub window_class: String,
    pub confidence: DialogConfidence,
    pub score: u8,
    pub evidence: Vec<String>,
    pub child_classes: Vec<String>,
}

#[cfg(not(target_os = "windows"))]
pub fn probe_foreground_dialog() -> Result<DialogProbeResult, String> {
    Err("文件对话框探针目前仅支持 Windows".to_string())
}

#[test]
#[cfg(target_os = "windows")]
fn test() {
    use std::thread;
    use std::time::Duration;

    // 手工测试持续 30 秒：期间切换普通窗口、打开/保存对话框观察输出。
    // 有限循环很重要，否则 cargo test / IDE 测试任务永远不会结束。
    const POLL_COUNT: usize = 60;
    let interval = Duration::from_millis(500);
    let mut last_signature = None;

    println!(
        "[DialogProbe] 手工测试已启动，将运行 {} 秒",
        POLL_COUNT as u64 * interval.as_millis() as u64 / 1000
    );

    for _ in 0..POLL_COUNT {
        match probe_foreground_dialog() {
            Ok(result) => {
                let signature = (result.hwnd, result.confidence, result.score);
                if last_signature != Some(signature) {
                    println!("[DialogProbe] {result:#?}");
                    last_signature = Some(signature);
                }
            }
            Err(error) => eprintln!("[DialogProbe] 探测失败: {error}"),
        }

        thread::sleep(interval);
    }

    println!("[DialogProbe] 手工测试结束");
}
