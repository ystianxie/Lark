use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    Light,
    Dark,
    Frosted,
    System,
    #[default]
    #[serde(other)]
    Classic,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appearance {
    #[serde(default)]
    pub theme: ThemeChoice,
}

#[tauri::command]
pub fn get_appearance() -> Result<Appearance, String> {
    super::config::read_appearance().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_appearance(app: AppHandle, appearance: Appearance) -> Result<(), String> {
    super::config::save_appearance_data(appearance.clone()).map_err(|error| error.to_string())?;
    // A persisted choice must remain successful even if a closing window cannot receive it.
    if let Err(error) = app.emit("appearance-changed", &appearance) {
        eprintln!("同步外观设置失败：{error}");
    }
    Ok(())
}

#[tauri::command]
pub async fn set_frosted_effect(
    window: tauri::WebviewWindow,
    enabled: bool,
) -> Result<bool, String> {
    // Only the transparent launcher uses desktop acrylic.
    if window.label() != "skylark" {
        return Ok(false);
    }
    #[cfg(windows)]
    {
        let (sender, mut receiver) = tauri::async_runtime::channel(1);
        let main_window = window.clone();
        window
            .run_on_main_thread(move || {
                // Only the frosted skin uses the native window shadow. Apply it
                // before Acrylic so its frame update cannot override the corners.
                let result = main_window
                    .set_shadow(enabled)
                    .map_err(|error| error.to_string())
                    .and_then(|_| main_window.hwnd().map_err(|error| error.to_string()))
                    .and_then(|hwnd| {
                        crate::utils::background::set_acrylic(
                            windows::Win32::Foundation::HWND(hwnd.0),
                            enabled,
                        )
                    });
                let _ = sender.try_send(result);
            })
            .map_err(|error| error.to_string())?;
        receiver
            .recv()
            .await
            .ok_or_else(|| "窗口关闭，无法更新磨砂效果".to_string())??;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BaseConfig;

    #[test]
    fn older_configs_default_to_classic_without_losing_settings() {
        let mut value = serde_json::to_value(BaseConfig::default()).unwrap();
        value.as_object_mut().unwrap().remove("appearance");
        value["hotkey_awaken"] = serde_json::json!("Ctrl+Shift+Space");
        let config: BaseConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.appearance.theme, ThemeChoice::Classic);
        assert_eq!(config.hotkey_awaken, "Ctrl+Shift+Space");
    }

    #[test]
    fn appearance_choices_round_trip_including_system_preference() {
        for theme in [
            ThemeChoice::Classic,
            ThemeChoice::Light,
            ThemeChoice::Dark,
            ThemeChoice::Frosted,
            ThemeChoice::System,
        ] {
            let appearance = Appearance { theme };
            let restored: Appearance =
                serde_json::from_str(&serde_json::to_string(&appearance).unwrap()).unwrap();
            assert_eq!(restored, appearance);
        }
    }

    #[test]
    fn unavailable_theme_falls_back_to_classic() {
        let appearance: Appearance = serde_json::from_str(r#"{"theme":"removed-skin"}"#).unwrap();
        assert_eq!(appearance.theme, ThemeChoice::Classic);
    }
}
