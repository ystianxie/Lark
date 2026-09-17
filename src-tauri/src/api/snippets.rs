use crate::api::clipboard::ClipboardOperator;
use crate::config::TextSnippet;
use enigo::{Direction, Enigo, Key as EnigoKey, Keyboard, Settings};
use rdev::{Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Clone, Default)]
struct SnippetSettings {
    enabled: bool,
    trigger: String,
    snippets: Vec<TextSnippet>,
}

#[derive(Default)]
struct MatchState {
    buffer: String,
    active: bool,
}

static SETTINGS: OnceLock<RwLock<SnippetSettings>> = OnceLock::new();
static INJECTING: AtomicBool = AtomicBool::new(false);

pub fn update_settings(enabled: bool, trigger: String, snippets: Vec<TextSnippet>) {
    let settings = SETTINGS.get_or_init(|| RwLock::new(SnippetSettings::default()));
    if let Ok(mut current) = settings.write() {
        *current = SnippetSettings {
            enabled,
            trigger,
            snippets,
        };
    }
}

#[cfg(target_os = "windows")]
pub fn start() {
    thread::spawn(|| {
        let mut state = MatchState::default();
        if let Err(error) = rdev::listen(move |event| handle_event(event, &mut state)) {
            eprintln!("文本片段键盘监听启动失败: {error:?}");
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start() {}

fn handle_event(event: Event, state: &mut MatchState) {
    if INJECTING.load(Ordering::Acquire) {
        return;
    }
    let EventType::KeyPress(key) = event.event_type else {
        return;
    };
    let Ok(settings) = SETTINGS
        .get_or_init(|| RwLock::new(SnippetSettings::default()))
        .read()
    else {
        return;
    };
    if !settings.enabled || settings.snippets.is_empty() {
        state.active = false;
        state.buffer.clear();
        return;
    }

    if key == Key::Backspace {
        if state.active && state.buffer.pop().is_none() {
            state.active = false;
        }
        return;
    }
    if matches!(
        key,
        Key::Escape
            | Key::Return
            | Key::Tab
            | Key::Space
            | Key::LeftArrow
            | Key::RightArrow
            | Key::UpArrow
            | Key::DownArrow
            | Key::Home
            | Key::End
            | Key::Delete
    ) {
        state.active = false;
        state.buffer.clear();
        return;
    }

    let Some(name) = event.name else { return };
    if !state.active {
        if name == settings.trigger {
            state.active = true;
            state.buffer.clear();
        }
        return;
    }
    if name.chars().count() != 1 {
        state.active = false;
        state.buffer.clear();
        return;
    }
    state.buffer.push_str(&name);

    if let Some(snippet) = settings
        .snippets
        .iter()
        .find(|snippet| snippet.keyword == state.buffer)
        .cloned()
    {
        let delete_count = settings.trigger.chars().count() + state.buffer.chars().count();
        state.active = false;
        state.buffer.clear();
        // rdev invokes us before Windows delivers the physical key to the target
        // control. Do not replace synchronously here, otherwise the last keyword
        // character can be deleted before it is rendered. A short delay lets the
        // original key event pass through first.
        let text = snippet.text.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(45));
            expand_snippet(delete_count, &text);
        });
    } else if !settings
        .snippets
        .iter()
        .any(|snippet| snippet.keyword.starts_with(&state.buffer))
    {
        state.active = false;
        state.buffer.clear();
    }
}

fn expand_snippet(delete_count: usize, text: &str) {
    INJECTING.store(true, Ordering::Release);
    let result = (|| -> anyhow::Result<()> {
        let mut enigo = Enigo::new(&Settings::default())?;
        for _ in 0..delete_count {
            enigo.key(EnigoKey::Backspace, Direction::Click)?;
        }
        ClipboardOperator::set_text_for_paste(text)?;
        thread::sleep(Duration::from_millis(20));
        enigo.key(EnigoKey::Control, Direction::Press)?;
        let paste_result = enigo.key(EnigoKey::Unicode('v'), Direction::Click);
        let release_result = enigo.key(EnigoKey::Control, Direction::Release);
        paste_result?;
        release_result?;
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("展开文本片段失败: {error}");
    }
    thread::sleep(Duration::from_millis(50));
    INJECTING.store(false, Ordering::Release);
}
