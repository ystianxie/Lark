mod config;
pub mod plugins;

pub use config::{
    app_settings, hotkey_settings, index_settings, save_index_settings_data, save_setting_data,
    save_snippet_settings_data, snippet_settings, ClipboardRetention, Config, TextSnippet,
};
