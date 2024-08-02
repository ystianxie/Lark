pub struct Config {
    clipboard_record_limit: Option<i32>,
}

impl Config {
    pub fn get_clipboard_record_limit() -> i32 {
        30
    }
    pub fn set_clipboard_record_shortcut(shortcut: &str) {
        //TODO
    }
}