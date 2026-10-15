//! 文件跳转复用键盘监听，不向系统注册或占用快捷键。
use rdev::{EventType, Key};
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

static BINDING: OnceLock<RwLock<Option<Shortcut>>> = OnceLock::new();

pub fn update_binding(binding: Option<Shortcut>) {
    *BINDING.get_or_init(|| RwLock::new(None)).write().unwrap() = binding;
}

#[derive(Default)]
pub struct KeyboardState {
    pressed: HashSet<Key>,
}

impl KeyboardState {
    // 无论功能是否启用都跟踪释放事件，左右修饰键分别记录。
    fn shortcut(&mut self, event: EventType) -> Option<Shortcut> {
        let key = match event {
            EventType::KeyRelease(key) => {
                self.pressed.remove(&key);
                return None;
            }
            EventType::KeyPress(key) if self.pressed.insert(key) => key,
            _ => return None,
        };
        let mut mods = Modifiers::empty();
        for (keys, modifier) in [
            ([Key::ControlLeft, Key::ControlRight], Modifiers::CONTROL),
            ([Key::ShiftLeft, Key::ShiftRight], Modifiers::SHIFT),
            ([Key::Alt, Key::AltGr], Modifiers::ALT),
            ([Key::MetaLeft, Key::MetaRight], Modifiers::SUPER),
        ] {
            if keys.iter().any(|key| self.pressed.contains(key)) {
                mods |= modifier;
            }
        }
        Some(Shortcut::new(Some(mods), key_code(key)?))
    }

    pub fn handle_event(&mut self, event: EventType, app: &tauri::AppHandle, injecting: bool) {
        let Some(shortcut) = self.shortcut(event) else {
            return;
        };
        if injecting || crate::HOTKEY_CAPTURE_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        if *BINDING.get_or_init(|| RwLock::new(None)).read().unwrap() != Some(shortcut) {
            return;
        }
        #[cfg(target_os = "windows")]
        {
            use tauri::Manager;

            // Hook 回调只做轻量匹配；路径缓存、文件框探测和导航统一由
            // navigator 处理，避免在按键线程里做两次探测导致切换瞬间丢按键。
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let jump = app.state::<crate::api::listary_jump::ListaryJumpHandle>();
                if let Err(error) = jump.trigger_ctrl_g() {
                    eprintln!("[ListaryJump] 快捷键触发失败：{error}");
                }
            });
        }
    }
}

fn key_code(key: Key) -> Option<Code> {
    // rdev 和 keyboard-types 的物理键名称多数一致。
    let name = match key {
        Key::ControlLeft
        | Key::ControlRight
        | Key::ShiftLeft
        | Key::ShiftRight
        | Key::Alt
        | Key::AltGr
        | Key::MetaLeft
        | Key::MetaRight
        | Key::Function => return None,
        Key::Return => "Enter".to_string(),
        Key::UpArrow => "ArrowUp".to_string(),
        Key::DownArrow => "ArrowDown".to_string(),
        Key::LeftArrow => "ArrowLeft".to_string(),
        Key::RightArrow => "ArrowRight".to_string(),
        Key::BackQuote => "Backquote".to_string(),
        Key::LeftBracket => "BracketLeft".to_string(),
        Key::RightBracket => "BracketRight".to_string(),
        Key::SemiColon => "Semicolon".to_string(),
        Key::BackSlash => "Backslash".to_string(),
        Key::Dot => "Period".to_string(),
        Key::KpReturn => "NumpadEnter".to_string(),
        Key::KpMinus => "NumpadSubtract".to_string(),
        Key::KpPlus => "NumpadAdd".to_string(),
        Key::KpMultiply => "NumpadMultiply".to_string(),
        Key::KpDivide => "NumpadDivide".to_string(),
        Key::KpDelete => "NumpadDecimal".to_string(),
        Key::Unknown(vk) if (0x7C..=0x87).contains(&vk) => format!("F{}", vk - 0x70 + 1),
        Key::Unknown(_) => return None,
        key => {
            let name = format!("{key:?}");
            if let Some(digit) = name.strip_prefix("Num") {
                format!("Digit{digit}")
            } else if let Some(digit) = name.strip_prefix("Kp") {
                format!("Numpad{digit}")
            } else {
                name
            }
        }
    };
    name.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_g_fires_once_until_g_is_released() {
        let mut state = KeyboardState::default();
        assert_eq!(state.shortcut(EventType::KeyPress(Key::ControlLeft)), None);
        assert_eq!(
            state.shortcut(EventType::KeyPress(Key::KeyG)),
            Some("Ctrl+G".parse().unwrap())
        );
        assert_eq!(state.shortcut(EventType::KeyPress(Key::KeyG)), None);
        state.shortcut(EventType::KeyRelease(Key::KeyG));
        assert_eq!(
            state.shortcut(EventType::KeyPress(Key::KeyG)),
            Some("Ctrl+G".parse().unwrap())
        );
    }

    #[test]
    fn releasing_one_ctrl_keeps_the_other_and_extra_modifiers_do_not_match() {
        let mut state = KeyboardState::default();
        state.shortcut(EventType::KeyPress(Key::ControlLeft));
        state.shortcut(EventType::KeyPress(Key::ControlRight));
        state.shortcut(EventType::KeyRelease(Key::ControlLeft));
        state.shortcut(EventType::KeyPress(Key::ShiftLeft));
        assert_eq!(
            state.shortcut(EventType::KeyPress(Key::KeyG)),
            Some("Ctrl+Shift+G".parse().unwrap())
        );
        state.shortcut(EventType::KeyRelease(Key::KeyG));
        state.shortcut(EventType::KeyRelease(Key::ShiftLeft));
        state.shortcut(EventType::KeyRelease(Key::ControlRight));
        assert_eq!(
            state.shortcut(EventType::KeyPress(Key::KeyG)).unwrap().mods,
            Modifiers::empty()
        );
    }

    #[test]
    fn maps_custom_shortcut_keys() {
        for (key, code) in [
            (Key::Num1, Code::Digit1),
            (Key::Kp2, Code::Numpad2),
            (Key::Return, Code::Enter),
            (Key::LeftArrow, Code::ArrowLeft),
            (Key::F8, Code::F8),
            (Key::Unknown(0x87), Code::F24),
        ] {
            assert_eq!(key_code(key), Some(code));
        }
    }
}
