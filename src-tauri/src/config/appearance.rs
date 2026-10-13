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

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FrostedBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
    scale: f64,
}

impl FrostedBounds {
    #[cfg(windows)]
    fn physical_region(&self) -> Result<[i32; 6], String> {
        let values = [
            self.x,
            self.y,
            self.width,
            self.height,
            self.radius,
            self.scale,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self.width == 0.0
            || self.height == 0.0
            || self.scale == 0.0
        {
            return Err("无效的磨砂窗口轮廓".into());
        }
        let diameter = 2.0 * self.radius.min(self.width / 2.0).min(self.height / 2.0);
        let pixels = [
            (self.x * self.scale).floor(),
            (self.y * self.scale).floor(),
            ((self.x + self.width) * self.scale).ceil() + 1.0,
            ((self.y + self.height) * self.scale).ceil() + 1.0,
            (diameter * self.scale).round(),
            (diameter * self.scale).round(),
        ];
        if pixels
            .iter()
            .any(|value| !value.is_finite() || *value >= i32::MAX as f64)
        {
            return Err("磨砂窗口轮廓超出范围".into());
        }
        Ok(pixels.map(|value| value as i32))
    }
}

#[cfg(windows)]
fn apply_frosted_bounds(
    window: &tauri::WebviewWindow,
    bounds: Option<FrostedBounds>,
) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn, HRGN};
    let hwnd = HWND(window.hwnd().map_err(|error| error.to_string())?.0);
    let region = if let Some(bounds) = bounds {
        let [left, top, right, bottom, diameter_x, diameter_y] = bounds.physical_region()?;
        let region =
            unsafe { CreateRoundRectRgn(left, top, right, bottom, diameter_x, diameter_y) };
        if region.is_invalid() {
            return Err("创建磨砂窗口轮廓失败".into());
        }
        region
    } else {
        HRGN::default()
    };
    // SetWindowRgn clips the native blur too, without resizing the window or adding a frame.
    // Windows takes ownership of the region only after a successful call.
    if unsafe { SetWindowRgn(hwnd, region, true) } == 0 {
        if !region.is_invalid() {
            unsafe {
                let _ = DeleteObject(region);
            }
        }
        return Err("更新磨砂窗口轮廓失败".into());
    }
    Ok(())
}

#[tauri::command]
pub fn update_frosted_bounds(
    window: tauri::WebviewWindow,
    bounds: FrostedBounds,
) -> Result<(), String> {
    if window.label() != "skylark" {
        return Ok(());
    }
    #[cfg(windows)]
    {
        apply_frosted_bounds(&window, Some(bounds))
    }
    #[cfg(not(windows))]
    {
        let _ = bounds;
        Ok(())
    }
}

#[tauri::command]
pub fn set_frosted_effect(
    window: tauri::WebviewWindow,
    enabled: bool,
    bounds: Option<FrostedBounds>,
) -> Result<bool, String> {
    // Only the transparent launcher uses desktop blur; notifications keep solid cards.
    if window.label() != "skylark" {
        return Ok(false);
    }
    #[cfg(windows)]
    {
        use tauri::window::{Color, Effect, EffectsBuilder};
        // Keep the launcher's original undecorated frame. A native shadow adds a border
        // and uses a different corner radius from the web content on Windows 11.
        window
            .set_shadow(false)
            .map_err(|error| error.to_string())?;
        if enabled && bounds.is_none() {
            return Err("缺少磨砂窗口轮廓".into());
        }
        apply_frosted_bounds(&window, if enabled { bounds } else { None })?;
        let effects = enabled.then(|| {
            EffectsBuilder::new()
                .effect(Effect::Blur)
                .color(Color(235, 242, 243, 40))
                .build()
        });
        if let Err(error) = window.set_effects(effects) {
            let _ = apply_frosted_bounds(&window, None);
            return Err(error.to_string());
        }
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = (enabled, bounds);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BaseConfig;

    #[cfg(windows)]
    fn launcher_bounds(scale: f64) -> FrostedBounds {
        FrostedBounds {
            x: 0.0,
            y: 0.0,
            width: 718.0,
            height: 600.0,
            radius: 15.0,
            scale,
        }
    }

    #[cfg(windows)]
    #[test]
    fn frosted_clip_scales_the_page_radius_and_covers_the_last_pixel() {
        use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, PtInRegion};
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let [left, top, right, bottom, dx, dy] =
                launcher_bounds(scale).physical_region().unwrap();
            let width = (718.0 * scale).ceil() as i32;
            let height = (600.0 * scale).ceil() as i32;
            assert_eq!(dx, (30.0 * scale).round() as i32);
            assert_eq!(dy, dx);
            unsafe {
                let region = CreateRoundRectRgn(left, top, right, bottom, dx, dy);
                assert!(!region.is_invalid());
                assert!(PtInRegion(region, width - 1, height / 2).as_bool());
                assert!(PtInRegion(region, width / 2, height - 1).as_bool());
                assert!(!PtInRegion(region, 0, 0).as_bool());
                assert!(!PtInRegion(region, width - 1, height - 1).as_bool());
                let _ = DeleteObject(region);
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn frosted_clip_tracks_compact_and_expanded_content_bounds() {
        let mut bounds = launcher_bounds(1.5);
        bounds.x = 0.25;
        bounds.y = 1.25;
        bounds.height = 71.54;
        assert_eq!(bounds.physical_region().unwrap(), [0, 1, 1079, 111, 45, 45]);
        bounds.height = 820.0;
        bounds.width = 980.0;
        assert_eq!(
            bounds.physical_region().unwrap(),
            [0, 1, 1472, 1233, 45, 45]
        );
    }

    #[cfg(windows)]
    #[test]
    fn frosted_clip_rejects_invalid_or_overflowing_geometry() {
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(launcher_bounds(scale).physical_region().is_err());
        }
        let mut bounds = launcher_bounds(1.0);
        bounds.width = 0.0;
        assert!(bounds.physical_region().is_err());
    }

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
