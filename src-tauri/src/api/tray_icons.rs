//! Persistent, best-effort replacement of existing Windows tray icons.
//! NotifyIconSettings supplies identities only; never modify Explorer's registry/cache.
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayRule {
    executable: String,
    tooltip: String,
    // Store decoded pixels, so moving/deleting the source image cannot break a rule.
    pixels: Vec<u8>,
    mask: Vec<u8>,
    custom: bool,
}

static RULES: OnceLock<Mutex<Result<Vec<TrayRule>, String>>> = OnceLock::new();

fn rules() -> &'static Mutex<Result<Vec<TrayRule>, String>> {
    RULES.get_or_init(|| {
        Mutex::new((|| {
            let path = crate::utils::dirs::app_data_dir()
                .map_err(|e| e.to_string())?
                .join("tray-icons.json");
            if !path.exists() {
                return Ok(Vec::new());
            }
            let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
            let rules: Vec<TrayRule> =
                serde_json::from_slice(&bytes).map_err(|e| format!("读取托盘规则失败：{e}"))?;
            if rules
                .iter()
                .any(|r| r.pixels.len() != 4096 || r.mask.len() != 128)
            {
                return Err("托盘规则中的图像数据无效".into());
            }
            Ok(rules)
        })())
    })
}

fn save_rules(updated: Vec<TrayRule>) -> Result<(), String> {
    let path = crate::utils::dirs::app_data_dir()
        .map_err(|e| e.to_string())?
        .join("tray-icons.json");
    write_rules(&path, &updated)
}

fn write_rules(path: &std::path::Path, updated: &[TrayRule]) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(updated).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[test]
    fn rules_survive_reload_update_and_removal_without_source_image() {
        let directory = std::env::temp_dir().join(format!(
            "lark-tray-rules-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("rules.json");
        let mut rule = TrayRule {
            executable: "C:\\Apps\\Weixin.exe".into(),
            tooltip: "微信".into(),
            pixels: vec![0; 4096],
            mask: vec![255; 128],
            custom: false,
        };
        write_rules(&path, &[rule.clone()]).unwrap();
        let loaded: Vec<TrayRule> = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(loaded == vec![rule.clone()]);
        rule.custom = true;
        rule.pixels[0] = 42;
        write_rules(&path, &[rule.clone()]).unwrap();
        let loaded: Vec<TrayRule> = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(loaded == vec![rule]);
        write_rules(&path, &[]).unwrap();
        let loaded: Vec<TrayRule> = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(loaded.is_empty());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}

#[tauri::command]
pub fn get_tray_icon_rules() -> Result<Vec<TrayRule>, String> {
    rules().lock().map_err(|e| e.to_string())?.clone()
}

#[tauri::command]
pub fn remove_tray_icon_rule(executable: String) -> Result<(), String> {
    let mut guard = rules().lock().map_err(|e| e.to_string())?;
    let mut updated = guard.as_ref().map_err(Clone::clone)?.clone();
    updated.retain(|r| !r.executable.eq_ignore_ascii_case(&executable));
    save_rules(updated.clone())?;
    *guard = Ok(updated);
    Ok(())
}

pub fn start_worker() {
    #[cfg(windows)]
    if let Err(error) = std::thread::Builder::new()
        .name("tray-icon-keeper".into())
        .spawn(native::keep_icons)
    {
        log::error!("启动托盘图标保持失败：{error}");
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayEntry {
    id: String,
    executable: String,
    tooltip: String,
}

#[tauri::command]
pub async fn list_tray_icons() -> Result<Vec<TrayEntry>, String> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(native::list)
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(not(windows))]
    Err("托盘图标替换暂时仅支持 Windows".into())
}

#[tauri::command]
pub async fn replace_tray_icon(id: String, image_path: Option<String>) -> Result<(), String> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(move || {
        native::replace(&id, image_path.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?;
    #[cfg(not(windows))]
    Err("托盘图标替换暂时仅支持 Windows".into())
}

#[cfg(windows)]
mod native {
    use super::{TrayEntry, TrayRule};
    use std::{collections::HashMap, mem::size_of};
    use windows::{
        core::{GUID, PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, BOOL, HINSTANCE, HWND, LPARAM},
            System::{
                Registry::*,
                Threading::{
                    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                    PROCESS_QUERY_LIMITED_INFORMATION,
                },
            },
            UI::{Shell::*, WindowsAndMessaging::*},
        },
    };

    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn open_key(parent: HKEY, path: &str) -> Option<Key> {
        let path = wide(path);
        let mut key = HKEY::default();
        unsafe {
            RegOpenKeyExW(parent, PCWSTR(path.as_ptr()), 0, KEY_READ, &mut key)
                .ok()
                .ok()?;
        }
        Some(Key(key))
    }
    fn value(key: HKEY, name: &str, flags: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
        let name = wide(name);
        let mut size = 0;
        unsafe {
            RegGetValueW(
                key,
                PCWSTR::null(),
                PCWSTR(name.as_ptr()),
                flags,
                None,
                None,
                Some(&mut size),
            )
            .ok()
            .ok()?;
            if size > 1024 * 1024 {
                return None;
            }
            let mut bytes = vec![0; size as usize];
            RegGetValueW(
                key,
                PCWSTR::null(),
                PCWSTR(name.as_ptr()),
                flags,
                None,
                Some(bytes.as_mut_ptr().cast()),
                Some(&mut size),
            )
            .ok()
            .ok()?;
            bytes.truncate(size as usize);
            Some(bytes)
        }
    }
    fn text(key: HKEY, name: &str) -> Option<String> {
        let bytes = value(key, name, RRF_RT_REG_SZ)?;
        let chars: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .take_while(|c| *c != 0)
            .collect();
        Some(String::from_utf16_lossy(&chars))
    }
    fn process_path(hwnd: HWND) -> Option<String> {
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            process_path_by_pid(pid)
        }
    }
    fn process_path_by_pid(pid: u32) -> Option<String> {
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = vec![0u16; 32768];
            let mut size = buf.len() as u32;
            let result = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(process);
            result.ok()?;
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        }
    }
    unsafe extern "system" fn collect(hwnd: HWND, context: LPARAM) -> BOOL {
        let windows = &mut *(context.0 as *mut Vec<HWND>);
        windows.push(hwnd);
        true.into()
    }
    struct Target {
        entry: TrayEntry,
        hwnd: HWND,
        uid: u32,
        guid: Option<GUID>,
    }
    fn targets() -> Result<Vec<Target>, String> {
        let root = open_key(HKEY_CURRENT_USER, "Control Panel\\NotifyIconSettings")
            .ok_or("当前 Windows 未提供可读取的托盘图标目录（此实验功能需要新版 Windows）")?;
        let mut handles = Vec::<HWND>::new();
        unsafe {
            EnumWindows(Some(collect), LPARAM(&mut handles as *mut _ as isize))
                .map_err(|e| e.to_string())?;
            // Some applications use message-only windows as tray owners.
            let mut after = HWND::default();
            while let Ok(hwnd) = FindWindowExW(HWND_MESSAGE, after, PCWSTR::null(), PCWSTR::null())
            {
                handles.push(hwnd);
                after = hwnd;
            }
        }
        let mut by_path: HashMap<String, Vec<HWND>> = HashMap::new();
        // A process may own many windows; query its executable only once per scan.
        let mut process_paths: HashMap<u32, Option<String>> = HashMap::new();
        for hwnd in handles {
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
            }
            if let Some(path) = process_paths
                .entry(pid)
                .or_insert_with(|| process_path_by_pid(pid))
            {
                by_path.entry(path.to_lowercase()).or_default().push(hwnd);
            }
        }
        let mut targets = Vec::new();
        for index in 0..10000 {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            let status = unsafe {
                RegEnumKeyExW(
                    root.0,
                    index,
                    PWSTR(name.as_mut_ptr()),
                    &mut len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            };
            if status == windows::Win32::Foundation::ERROR_NO_MORE_ITEMS {
                break;
            }
            status.ok().map_err(|e| e.to_string())?;
            let id = String::from_utf16_lossy(&name[..len as usize]);
            let Some(key) = open_key(root.0, &id) else {
                continue;
            };
            let Some(executable) = text(key.0, "ExecutablePath") else {
                continue;
            };
            let guid = text(key.0, "IconGuid").and_then(|s| parse_guid(&s));
            let uid = value(key.0, "UID", RRF_RT_REG_DWORD)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_le_bytes);
            if guid.is_none() && uid.is_none() {
                continue;
            }
            let Some(handles) = by_path.get(&executable.to_lowercase()) else {
                continue;
            };
            for hwnd in handles {
                let identifier = NOTIFYICONIDENTIFIER {
                    cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
                    hWnd: *hwnd,
                    uID: uid.unwrap_or(0),
                    guidItem: guid.unwrap_or_default(),
                };
                // Filter stale registry entries and validate the live (window, UID) identity.
                if unsafe { Shell_NotifyIconGetRect(&identifier) }.is_err() {
                    continue;
                }
                targets.push(Target {
                    entry: TrayEntry {
                        id: id.clone(),
                        executable: executable.clone(),
                        tooltip: text(key.0, "InitialTooltip").unwrap_or_default(),
                    },
                    hwnd: *hwnd,
                    uid: uid.unwrap_or(0),
                    guid,
                });
                break;
            }
        }
        Ok(targets)
    }
    pub fn list() -> Result<Vec<TrayEntry>, String> {
        Ok(targets()?.into_iter().map(|t| t.entry).collect())
    }

    fn parse_guid(value: &str) -> Option<GUID> {
        if !value.is_ascii() {
            return None;
        }
        let value = value.trim().trim_matches(['{', '}']);
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 5 || parts.iter().any(|part| part.is_empty()) {
            return None;
        }
        let data1 = u32::from_str_radix(parts[0], 16).ok()?;
        let data2 = u16::from_str_radix(parts[1], 16).ok()?;
        let data3 = u16::from_str_radix(parts[2], 16).ok()?;
        if parts[3].len() != 4 || parts[4].len() != 12 {
            return None;
        }
        let tail = format!("{}{}", parts[3], parts[4]);
        let mut data4 = [0u8; 8];
        for (i, byte) in data4.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&tail[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(GUID {
            data1,
            data2,
            data3,
            data4,
        })
    }
    fn icon_pixels(path: Option<&str>) -> Result<(Vec<u8>, Vec<u8>), String> {
        let rgba = if let Some(path) = path {
            let reader =
                image::ImageReader::open(path).map_err(|e| format!("无法读取图片：{e}"))?;
            let mut reader = reader.with_guessed_format().map_err(|e| e.to_string())?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            reader.limits(limits);
            reader
                .decode()
                .map_err(|e| format!("无法解析图片：{e}"))?
                .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
                .to_rgba8()
                .into_raw()
        } else {
            vec![0; 32 * 32 * 4]
        };
        let mut bgra = vec![0; rgba.len()];
        let mut mask = vec![0u8; 32 * 4];
        for (i, p) in rgba.chunks_exact(4).enumerate() {
            let alpha = p[3] as u16;
            bgra[i * 4..i * 4 + 4].copy_from_slice(&[
                (p[2] as u16 * alpha / 255) as u8,
                (p[1] as u16 * alpha / 255) as u8,
                (p[0] as u16 * alpha / 255) as u8,
                p[3],
            ]);
            if alpha == 0 {
                mask[i / 8] |= 0x80 >> (i % 8);
            }
        }
        Ok((bgra, mask))
    }
    pub fn replace(id: &str, path: Option<&str>) -> Result<(), String> {
        let target = targets()?
            .into_iter()
            .find(|t| t.entry.id == id)
            .ok_or("图标已退出或无法定位，请刷新后重试")?;
        let (pixels, mask) = icon_pixels(path)?;
        let rule = TrayRule {
            executable: target.entry.executable.clone(),
            tooltip: target.entry.tooltip.clone(),
            pixels,
            mask,
            custom: path.is_some(),
        };
        let icon = make_icon(&rule)?;
        apply(&target, icon.0)?;
        let mut guard = super::rules().lock().map_err(|e| e.to_string())?;
        let mut updated = guard.as_ref().map_err(Clone::clone)?.clone();
        updated.retain(|r| !r.executable.eq_ignore_ascii_case(&rule.executable));
        updated.push(rule);
        super::save_rules(updated.clone())?;
        *guard = Ok(updated);
        Ok(())
    }

    struct Icon(HICON);
    impl Drop for Icon {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }
    fn make_icon(rule: &TrayRule) -> Result<Icon, String> {
        unsafe {
            CreateIcon(
                HINSTANCE::default(),
                32,
                32,
                1,
                32,
                rule.mask.as_ptr(),
                rule.pixels.as_ptr(),
            )
            .map(Icon)
            .map_err(|e| e.to_string())
        }
    }
    fn apply(target: &Target, icon: HICON) -> Result<(), String> {
        unsafe {
            let mut data = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: target.hwnd,
                uID: target.uid,
                uFlags: NIF_ICON,
                hIcon: icon,
                ..Default::default()
            };
            if let Some(guid) = target.guid {
                data.uFlags |= NIF_GUID;
                data.guidItem = guid;
            }
            let result = Shell_NotifyIconW(NIM_MODIFY, &data).as_bool();
            if result {
                Ok(())
            } else {
                Err("Windows 拒绝了此次图标替换，此应用或系统版本可能不支持跨应用修改".into())
            }
        }
    }

    pub fn keep_icons() {
        use std::time::{Duration, Instant};
        let mut cached = Vec::new();
        let mut last_scan = Instant::now() - Duration::from_secs(5);
        let mut previous = Vec::new();
        let mut icons = Vec::<(String, Icon)>::new();
        loop {
            // Serialize with save/removal so a removed rule cannot be reapplied afterwards.
            if let Ok(guard) = super::rules().lock() {
                if let Ok(rules) = guard.as_ref() {
                    if *rules != previous {
                        icons = rules
                            .iter()
                            .filter_map(|r| match make_icon(r) {
                                Ok(icon) => Some((r.executable.clone(), icon)),
                                Err(e) => {
                                    log::warn!("创建保持图标失败：{e}");
                                    None
                                }
                            })
                            .collect();
                        previous = rules.clone();
                        last_scan = Instant::now() - Duration::from_secs(5);
                    }
                    if !icons.is_empty() {
                        if last_scan.elapsed() >= Duration::from_secs(3) {
                            match targets() {
                                Ok(targets) => cached = targets,
                                Err(e) => {
                                    cached.clear();
                                    log::debug!("重新定位托盘图标失败：{e}");
                                }
                            }
                            last_scan = Instant::now();
                        }
                        for target in &cached {
                            if let Some((_, icon)) = icons.iter().find(|(path, _)| {
                                path.eq_ignore_ascii_case(&target.entry.executable)
                            }) {
                                // Cached HWNDs can be reused by another process after restart.
                                if process_path(target.hwnd).is_some_and(|p| {
                                    p.eq_ignore_ascii_case(&target.entry.executable)
                                }) {
                                    let _ = apply(target, icon.0);
                                }
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn transparent_icon_has_clear_pixels_and_full_and_mask() {
            let (pixels, mask) = icon_pixels(None).unwrap();
            assert_eq!(pixels, vec![0; 4096]);
            assert_eq!(mask, vec![255; 128]);
        }
    }
}
