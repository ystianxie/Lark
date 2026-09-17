//! Windows 文件对话框路径设置器。
//!
//! 策略顺序：
//! 1. 优先通过窗口消息直接修改标准文件对话框的文件名 Edit 控件；
//! 2. 找不到安全的目标控件时，才回退到 `Ctrl+L -> 路径 -> Enter`。
//!
//! 第一种方式不会逐字注入路径，也不需要占用剪贴板，对用户输入的干扰更小。

#[cfg(target_os = "windows")]
mod platform {
    use std::mem::size_of;
    use std::path::Path;
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;

    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        VIRTUAL_KEY, VK_CONTROL, VK_L, VK_RETURN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetDlgCtrlID, GetForegroundWindow, IsWindow,
        IsWindowVisible, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_CHAR, WM_GETTEXT,
        WM_GETTEXTLENGTH, WM_KEYDOWN, WM_KEYUP, WM_SETTEXT,
    };

    use crate::api::dialog_probe::{probe_foreground_dialog, DialogConfidence};

    const FILE_NAME_EDIT_ID: i32 = 0x480; // Windows SDK: edt1
    const MESSAGE_TIMEOUT_MS: u32 = 500;
    static NAVIGATION_LOCK: Mutex<()> = Mutex::new(());

    /// 本次导航实际采用的策略。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DialogNavigationMethod {
        /// 通过 WM_SETTEXT 定向修改文件名 Edit 控件，并向该控件发送 Enter。
        FileNameEdit,
        /// 未找到可安全操作的文件名控件，回退到全局键盘输入。
        KeyboardFallback,
    }

    /// 手工测试入口：等待 3 秒后设置前台文件对话框路径。
    ///
    /// 调用后请在 3 秒内把目标文件对话框切到前台。
    pub fn test_set_dialog_path(path: impl AsRef<Path>) -> Result<DialogNavigationMethod, String> {
        let path = path.as_ref().to_path_buf();
        println!(
            "[DialogNavigator] 请在 3 秒内切换到文件对话框，目标路径: {}",
            path.display()
        );
        thread::sleep(Duration::from_secs(3));

        let method = navigate_foreground_dialog(&path)?;
        println!("[DialogNavigator] 已提交导航，使用策略: {method:?}");
        Ok(method)
    }

    /// 兼容旧调用方式：设置当前前台文件对话框目录。
    ///
    /// `Ok(())` 表示导航操作已成功提交给目标窗口，不代表已经验证最终目录。
    pub fn set_foreground_dialog_path(path: impl AsRef<Path>) -> Result<(), String> {
        navigate_foreground_dialog(path).map(|_| ())
    }

    /// 设置当前前台文件对话框目录，并返回实际采用的策略。
    pub fn navigate_foreground_dialog(
        path: impl AsRef<Path>,
    ) -> Result<DialogNavigationMethod, String> {
        // 热键或监听器可能在短时间内重复触发。拒绝并发导航，避免两次操作互相覆盖。
        let _guard = NAVIGATION_LOCK
            .try_lock()
            .map_err(|_| "已有文件对话框导航正在执行".to_string())?;

        let path = path.as_ref();
        if !path.is_dir() {
            return Err(format!("目标路径不存在或不是目录: {}", path.display()));
        }

        let path_text = path
            .to_str()
            .ok_or_else(|| format!("目标路径不是有效 Unicode: {}", path.display()))?;

        let probe = probe_foreground_dialog()?;
        if probe.confidence != DialogConfidence::Confirmed {
            return Err(format!(
                "当前窗口未被确认为文件对话框: {:?}, class={:?}, title={:?}",
                probe.confidence, probe.window_class, probe.title
            ));
        }

        let dialog_hwnd = HWND(probe.hwnd as *mut _);
        ensure_foreground(dialog_hwnd)?;

        match try_navigate_via_file_name_edit(dialog_hwnd, path_text) {
            Ok(()) => Ok(DialogNavigationMethod::FileNameEdit),
            Err(direct_error) => {
                ensure_foreground(dialog_hwnd)?;
                eprintln!("[DialogNavigator] 文件名控件直达不可用，回退键盘方案: {direct_error}");
                navigate_via_keyboard(dialog_hwnd, path_text)?;
                Ok(DialogNavigationMethod::KeyboardFallback)
            }
        }
    }

    /// 直接向标准文件名输入框写入 `目录\` 并发送 Enter。
    fn try_navigate_via_file_name_edit(dialog: HWND, path: &str) -> Result<(), String> {
        let edit = find_file_name_edit(dialog)?;
        let original_text = get_control_text(edit)?;
        let navigation_text = format!("{}{}", path.trim_end_matches(['\\', '/']), "\\");

        set_control_text(edit, &navigation_text)?;

        // 文件对话框通常会把文件名框中的 `目录\` 解释为目录导航。
        // Enter 只定向发送到该 Edit，不向当前全局焦点注入字符。
        if let Err(error) = send_enter_to_control(edit) {
            let _ = set_control_text(edit, &original_text);
            return Err(error);
        }

        thread::sleep(Duration::from_millis(120));

        // 对话框仍存在时恢复原文件名，避免破坏“另存为”中的用户输入。
        if unsafe { IsWindow(dialog).as_bool() } && unsafe { IsWindow(edit).as_bool() } {
            set_control_text(edit, &original_text)
                .map_err(|error| format!("目录导航已提交，但恢复原文件名失败: {error}"))?;
        }

        Ok(())
    }

    /// 优先使用标准 Common Dialog 的 edt1(0x480)。找不到时，只在恰好存在
    /// 一个可见 Edit 控件的情况下使用它，避免误写地址栏或搜索框。
    fn find_file_name_edit(dialog: HWND) -> Result<HWND, String> {
        let mut state = EditSearchState::default();
        let parameter = LPARAM((&mut state as *mut EditSearchState) as isize);
        unsafe { EnumChildWindows(dialog, Some(enum_edit_proc), parameter) };

        if let Some(edit) = state.file_name_edit {
            return Ok(edit);
        }
        if state.visible_edits.len() == 1 {
            return Ok(state.visible_edits[0]);
        }

        Err(format!(
            "未找到唯一且安全的文件名 Edit 控件（可见 Edit 数量: {}）",
            state.visible_edits.len()
        ))
    }

    #[derive(Default)]
    struct EditSearchState {
        file_name_edit: Option<HWND>,
        visible_edits: Vec<HWND>,
    }

    unsafe extern "system" fn enum_edit_proc(hwnd: HWND, parameter: LPARAM) -> BOOL {
        let state = &mut *(parameter.0 as *mut EditSearchState);
        if !IsWindowVisible(hwnd).as_bool() || window_class(hwnd) != "Edit" {
            return BOOL(1);
        }

        if GetDlgCtrlID(hwnd) == FILE_NAME_EDIT_ID {
            state.file_name_edit = Some(hwnd);
        }
        state.visible_edits.push(hwnd);
        BOOL(1)
    }

    fn window_class(hwnd: HWND) -> String {
        let mut buffer = [0u16; 128];
        let length = unsafe { GetClassNameW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }

    fn get_control_text(hwnd: HWND) -> Result<String, String> {
        let length = send_message(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0))? as usize;
        let mut buffer = vec![0u16; length.saturating_add(1)];
        send_message(
            hwnd,
            WM_GETTEXT,
            WPARAM(buffer.len()),
            LPARAM(buffer.as_mut_ptr() as isize),
        )?;
        let end = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        Ok(String::from_utf16_lossy(&buffer[..end]))
    }

    fn set_control_text(hwnd: HWND, text: &str) -> Result<(), String> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let result = send_message(hwnd, WM_SETTEXT, WPARAM(0), LPARAM(wide.as_ptr() as isize))?;
        if result == 0 {
            return Err("目标控件拒绝 WM_SETTEXT".to_string());
        }
        Ok(())
    }

    fn send_enter_to_control(hwnd: HWND) -> Result<(), String> {
        send_message(hwnd, WM_KEYDOWN, WPARAM(VK_RETURN.0 as usize), LPARAM(0))?;
        send_message(hwnd, WM_CHAR, WPARAM(VK_RETURN.0 as usize), LPARAM(0))?;
        send_message(hwnd, WM_KEYUP, WPARAM(VK_RETURN.0 as usize), LPARAM(0))?;
        Ok(())
    }

    fn send_message(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Result<isize, String> {
        let mut result = 0usize;
        let status = unsafe {
            SendMessageTimeoutW(
                hwnd,
                message,
                wparam,
                lparam,
                SMTO_ABORTIFHUNG,
                MESSAGE_TIMEOUT_MS,
                Some(&mut result),
            )
        };
        if status.0 == 0 {
            return Err(format!("窗口消息 0x{message:X} 超时或发送失败"));
        }
        Ok(result as isize)
    }

    fn navigate_via_keyboard(dialog: HWND, path: &str) -> Result<(), String> {
        ensure_foreground(dialog)?;
        send_shortcut(VK_CONTROL, VK_L)?;
        thread::sleep(Duration::from_millis(120));
        ensure_foreground(dialog)?;
        send_unicode_text(path)?;
        thread::sleep(Duration::from_millis(80));
        ensure_foreground(dialog)?;
        send_key(VK_RETURN)
    }

    fn ensure_foreground(expected: HWND) -> Result<(), String> {
        let current = unsafe { GetForegroundWindow() };
        if current != expected {
            return Err("操作期间前台窗口发生变化，已取消路径输入".to_string());
        }
        Ok(())
    }

    fn send_shortcut(modifier: VIRTUAL_KEY, key: VIRTUAL_KEY) -> Result<(), String> {
        send_inputs(&[
            virtual_key_input(modifier, false),
            virtual_key_input(key, false),
            virtual_key_input(key, true),
            virtual_key_input(modifier, true),
        ])
    }

    fn send_key(key: VIRTUAL_KEY) -> Result<(), String> {
        send_inputs(&[virtual_key_input(key, false), virtual_key_input(key, true)])
    }

    fn send_unicode_text(text: &str) -> Result<(), String> {
        let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
        for code_unit in text.encode_utf16() {
            inputs.push(unicode_input(code_unit, false));
            inputs.push(unicode_input(code_unit, true));
        }
        send_inputs(&inputs)
    }

    fn send_inputs(inputs: &[INPUT]) -> Result<(), String> {
        if inputs.is_empty() {
            return Ok(());
        }
        let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            return Err(format!(
                "SendInput 仅发送了 {sent}/{} 个输入事件，可能受到权限级别限制",
                inputs.len()
            ));
        }
        Ok(())
    }

    fn virtual_key_input(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: 0,
                    dwFlags: if key_up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn unicode_input(code_unit: u16, key_up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code_unit,
                    dwFlags: if key_up {
                        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                    } else {
                        KEYEVENTF_UNICODE
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}

#[cfg(target_os = "windows")]
pub use platform::{
    navigate_foreground_dialog, set_foreground_dialog_path, test_set_dialog_path,
    DialogNavigationMethod,
};

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogNavigationMethod {
    FileNameEdit,
    KeyboardFallback,
}

#[cfg(not(target_os = "windows"))]
pub fn set_foreground_dialog_path(_path: impl AsRef<std::path::Path>) -> Result<(), String> {
    Err("文件对话框路径设置目前仅支持 Windows".to_string())
}
