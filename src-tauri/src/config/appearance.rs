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
    #[serde(default)]
    pub wallpaper: Wallpaper,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Wallpaper {
    pub path: String,
    pub enabled: bool,
    pub scale: u16,
    pub x: u16,
    pub y: u16,
    pub opacity: u16,
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            path: String::new(),
            enabled: false,
            scale: 100,
            x: 50,
            y: 50,
            opacity: 35,
        }
    }
}

impl Wallpaper {
    fn normalize(&mut self) {
        self.scale = self.scale.clamp(100, 300);
        self.x = self.x.min(100);
        self.y = self.y.min(100);
        self.opacity = self.opacity.min(100);
        self.enabled &= !self.path.is_empty();
    }
}

#[tauri::command]
pub async fn import_wallpaper(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::hash::{Hash, Hasher};
        use std::io::Read;
        let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
        let mut bytes = Vec::new();
        file.take(20 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() > 20 * 1024 * 1024 {
            return Err("图片不能超过 20 MB".into());
        }
        let format = image::guess_format(&bytes).map_err(|_| "无法识别图片格式")?;
        if !matches!(
            format,
            image::ImageFormat::Png
                | image::ImageFormat::Jpeg
                | image::ImageFormat::WebP
                | image::ImageFormat::Bmp
        ) {
            return Err("请选择 PNG、JPG、WebP 或 BMP 图片".into());
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hash);
        let directory = crate::utils::dirs::app_data_dir()
            .map_err(|error| error.to_string())?
            .join("wallpapers");
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let extension = match format {
            image::ImageFormat::Png => "png",
            image::ImageFormat::Jpeg => "jpg",
            image::ImageFormat::WebP => "webp",
            image::ImageFormat::Bmp => "bmp",
            _ => unreachable!(),
        };
        let destination = directory.join(format!("{:016x}.{extension}", hash.finish()));
        if !destination.exists() {
            std::fs::write(&destination, &bytes).map_err(|error| error.to_string())?;
        }
        Ok(destination.to_string_lossy().into_owned())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_appearance() -> Result<Appearance, String> {
    super::config::read_appearance().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_appearance(app: AppHandle, mut appearance: Appearance) -> Result<(), String> {
    appearance.wallpaper.normalize();
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
            let appearance = Appearance {
                theme,
                ..Appearance::default()
            };
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

    #[test]
    fn older_appearance_defaults_to_disabled_wallpaper() {
        let appearance: Appearance = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(appearance.wallpaper, Wallpaper::default());
    }

    #[test]
    fn wallpaper_round_trips_and_bounds_adjustments() {
        let mut wallpaper = Wallpaper {
            path: "wallpapers/test.png".into(),
            enabled: true,
            scale: 900,
            x: 250,
            y: 75,
            opacity: 200,
        };
        wallpaper.normalize();
        assert_eq!(
            (wallpaper.scale, wallpaper.x, wallpaper.y, wallpaper.opacity),
            (300, 100, 75, 100)
        );
        let restored: Wallpaper =
            serde_json::from_str(&serde_json::to_string(&wallpaper).unwrap()).unwrap();
        assert_eq!(restored, wallpaper);
        wallpaper.path.clear();
        wallpaper.normalize();
        assert!(!wallpaper.enabled);
    }
}
