mod config;
pub mod plugins;

pub use config::{
    app_settings, clear_plugin_settings_data, ensure_file_search_paths_initialized,
    hotkey_settings, index_initialization_flags_present, index_settings, plugin_settings,
    plugin_settings_map, save_index_initialization_flags, save_index_settings_data,
    save_plugin_settings_data, save_setting_data, save_snippet_settings_data, snippet_settings,
    BaseConfig, ClipboardRetention, Config, TextSnippet,
};
