//! Native text selection access for the translation shortcut.
//!
//! On Windows the reader first uses UI Automation's TextPattern and falls
//! back to copying the active selection for controls (such as browsers) that
//! do not expose a usable text pattern. On macOS the Accessibility API
//! exposes AXSelectedText on the focused UI element.

#[cfg(target_os = "windows")]
pub fn read_selected_text() -> Result<Option<String>, String> {
    use crate::api::clipboard::ClipboardOperator;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::thread;
    use std::time::{Duration, Instant};
    use uiautomation::{patterns::UITextPattern, UIAutomation};

    fn wait_for_hotkey_modifiers_release() {
        use winapi::um::winuser::{
            GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
            VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
        };

        let keys = [
            VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_MENU, VK_LMENU, VK_RMENU, VK_SHIFT,
            VK_LSHIFT, VK_RSHIFT, VK_LWIN, VK_RWIN,
        ];
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            let modifier_down = keys
                .iter()
                .any(|key| unsafe { GetAsyncKeyState(*key) } < 0);
            if !modifier_down {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    // UIA is the least invasive path, but it is not implemented by a number
    // of Chromium/Electron/WebView controls. Retry briefly because some
    // providers publish the selection a few milliseconds after the hotkey
    // callback runs, then fall back to the universally supported Ctrl+C path.
    for _ in 0..3 {
        if let Ok(automation) = UIAutomation::new() {
            if let Ok(walker) = automation.get_raw_view_walker() {
                if let Ok(mut element) = automation.get_focused_element() {
                    // Some applications expose TextPattern on a parent
                    // container instead of the focused child. Do not stop at
                    // an arbitrary shallow depth; browser accessibility trees
                    // are commonly deeper than eight nodes.
                    for _ in 0..64 {
                        if let Ok(pattern) = element.get_pattern::<UITextPattern>() {
                            if let Ok(ranges) = pattern.get_selection() {
                                let mut selected = String::new();
                                for range in ranges {
                                    if let Ok(text) = range.get_text(-1) {
                                        selected.push_str(&text);
                                    }
                                }

                                if !selected.trim().is_empty() {
                                    return Ok(Some(selected));
                                }
                            }
                        }

                        let Ok(parent) = walker.get_parent(&element) else {
                            break;
                        };
                        element = parent;
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }

    // A large class of text controls (notably Chrome, Edge and Electron)
    // expose no usable TextPattern selection. Copying the active selection is
    // their reliable interoperability mechanism. Restore a text clipboard
    // snapshot immediately so invoking translation does not unexpectedly
    // replace the user's clipboard contents.
    // The global-shortcut callback runs on key-down; wait for Ctrl/Alt/etc.
    // to be released so the synthetic Ctrl+C is not sent as Ctrl+Alt+C.
    wait_for_hotkey_modifiers_release();
    let previous_clipboard = ClipboardOperator::get_text().ok();
    let previous_sequence = crate::api::clipboard::get_clipboard_sequence_number();
    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|error| error.to_string())?;
    let copy_result = enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|error| error.to_string());
    let release_result = enigo
        .key(Key::Control, Direction::Release)
        .map_err(|error| error.to_string());
    if let Err(error) = copy_result.or(release_result) {
        if let Some(previous) = previous_clipboard {
            let _ = ClipboardOperator::set_text(&previous);
        }
        return Err(error);
    }

    // Clipboard providers update asynchronously. Keep this short so the
    // selected text is available before the panel is shown.
    thread::sleep(Duration::from_millis(60));
    let current_sequence = crate::api::clipboard::get_clipboard_sequence_number();
    let selected = if current_sequence != 0 && current_sequence != previous_sequence {
        ClipboardOperator::get_text()
            .ok()
            .filter(|text| !text.trim().is_empty())
    } else {
        None
    };

    if let Some(previous) = previous_clipboard {
        if let Err(error) = ClipboardOperator::set_text(&previous) {
            eprintln!("[Translation] 恢复原剪贴板失败: {error}");
        }
    }

    Ok(selected)
}

#[cfg(target_os = "macos")]
pub fn read_selected_text() -> Result<Option<String>, String> {
    use accessibility::{AXAttribute, AXUIElement};
    use core_foundation::{base::CFType, string::CFString};

    let system = AXUIElement::system_wide();
    let focused_attribute =
        AXAttribute::<CFType>::new(&CFString::from_static_string("AXFocusedUIElement"));
    let selected_attribute =
        AXAttribute::<CFType>::new(&CFString::from_static_string("AXSelectedText"));

    let focused_value = system
        .attribute(&focused_attribute)
        .map_err(|error| error.to_string())?;
    let Some(focused) = focused_value.downcast::<AXUIElement>() else {
        return Ok(None);
    };

    let selected = focused
        .attribute(&selected_attribute)
        .ok()
        .and_then(|value| value.downcast::<CFString>())
        .map(|value| value.to_string());

    Ok(selected.filter(|text| !text.trim().is_empty()))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn read_selected_text() -> Result<Option<String>, String> {
    Ok(None)
}
