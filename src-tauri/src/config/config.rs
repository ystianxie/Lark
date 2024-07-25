pub struct Config {
    clipboard_record_limit: i64,
}

impl Config {
    pub fn get_clipboard_record_limit() -> i64 {
        30
    }
}