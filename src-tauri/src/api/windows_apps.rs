use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use log::debug;
use windows::{
    core::{w, Interface, PCWSTR},
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
        },
        System::Com::{
            CoCreateInstance, CoInitialize, CoTaskMemFree, CoUninitialize, CreateBindCtx,
            IPersistFile,
            StructuredStorage::{PropVariantClear, PropVariantToString},
            CLSCTX_INPROC_SERVER, STGM_READ,
        },
        UI::Shell::{
            BHID_EnumItems, BHID_PropertyStore, IEnumShellItems, IShellItem,
            IShellItemImageFactory, IShellLinkW,
            PropertiesSystem::{
                IPropertyStore, PSGetNameFromPropertyKey, PSGetPropertyKeyFromName, PROPERTYKEY,
            },
            SHCreateItemFromParsingName, SHGetNameFromIDList, ShellLink,
            SIGDN_DESKTOPABSOLUTEPARSING, SIIGBF_ICONONLY, SLGP_RAWPATH,
        },
    },
};

#[derive(Default)]
pub struct RegisteredApp {
    pub name: String,
    pub icon: String,
    pub start: String,
    pub executable: String,
    // None means unavailable; Some("") means confirmed to have no arguments.
    pub arguments: Option<String>,
}

pub struct ShortcutInfo {
    pub target: String,
    pub arguments: Option<String>,
    pub working_directory: String,
}

/// Read metadata only. Do not Resolve/execute the link or contact its target.
pub fn read_shortcut(path: &str) -> Result<(String, Option<String>), String> {
    let info = read_shortcut_details(path)?;
    Ok((info.target, info.arguments))
}

pub fn read_shortcut_details(path: &str) -> Result<ShortcutInfo, String> {
    unsafe {
        CoInitialize(None).ok().map_err(|error| error.to_string())?;
        let result = (|| -> windows::core::Result<ShortcutInfo> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let file: IPersistFile = link.cast()?;
            let path: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
            file.Load(PCWSTR(path.as_ptr()), STGM_READ)?;
            let mut target = vec![0u16; 32768];
            let mut arguments = vec![0u16; 32768];
            let mut working_directory = vec![0u16; 32768];
            let mut target = if link
                .GetPath(&mut target, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
                .is_ok()
            {
                read_wide_buffer(&target)
            } else {
                String::new()
            };
            // GetPath only describes filesystem targets. Virtual Shell entries
            // (Run, Control Panel, Explorer, etc.) keep their target in a PIDL.
            if target.is_empty() {
                if let Ok(pidl) = link.GetIDList() {
                    if !pidl.is_null() {
                        if let Ok(name) = SHGetNameFromIDList(pidl, SIGDN_DESKTOPABSOLUTEPARSING) {
                            target = name.to_string().unwrap_or_default();
                            CoTaskMemFree(Some(name.0.cast()));
                        }
                        CoTaskMemFree(Some(pidl.cast()));
                    }
                }
            }
            let arguments =
                if link.GetArguments(&mut arguments).is_ok() && arguments.last() == Some(&0) {
                    Some(read_wide_buffer(&arguments))
                } else {
                    None
                };
            let working_directory = if link.GetWorkingDirectory(&mut working_directory).is_ok() {
                read_wide_buffer(&working_directory)
            } else {
                String::new()
            };
            Ok(ShortcutInfo {
                target,
                arguments,
                working_directory,
            })
        })();
        CoUninitialize();
        result.map_err(|error| error.to_string())
    }
}

/// Windows Installer advertised shortcuts store a DarwinDataBlock in the
/// `.lnk`. Their target can be an `.ico` (or another non-executable path),
/// while Windows Shell still launches the installed application correctly.
pub fn is_advertised_shortcut(path: &str) -> bool {
    const DARWIN_SIGNATURE: u32 = 0xA0000006;
    const DARWIN_BLOCK_SIZE: usize = 0x314;
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    bytes.windows(8).enumerate().any(|(offset, block)| {
        let size = u32::from_le_bytes([block[0], block[1], block[2], block[3]]) as usize;
        let signature = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
        signature == DARWIN_SIGNATURE
            && size == DARWIN_BLOCK_SIZE
            && offset.saturating_add(size) <= bytes.len()
    })
}

fn read_wide_buffer(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// Ask the Shell for an item's icon, including virtual namespaces and AppsFolder.
/// File icon APIs treat these identifiers as filenames and cannot read them.
pub fn read_shell_icon_base64(path: &str) -> Result<String, String> {
    unsafe {
        CoInitialize(None).ok().map_err(|error| error.to_string())?;
        let result = (|| -> Result<String, String> {
            let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
            let factory: IShellItemImageFactory =
                SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
                    .map_err(|error| error.to_string())?;
            let bitmap = factory
                .GetImage(SIZE { cx: 64, cy: 64 }, SIIGBF_ICONONLY)
                .map_err(|error| error.to_string())?;
            let result = shell_bitmap_to_base64(bitmap);
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            result
        })();
        CoUninitialize();
        result
    }
}

unsafe fn shell_bitmap_to_base64(bitmap: HBITMAP) -> Result<String, String> {
    let mut details = BITMAP::default();
    if GetObjectW(
        HGDIOBJ(bitmap.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some((&mut details as *mut BITMAP).cast()),
    ) == 0
        || details.bmWidth <= 0
        || details.bmHeight <= 0
        || details.bmWidth > 1024
        || details.bmHeight > 1024
    {
        return Err("Shell 图标位图尺寸无效".to_string());
    }
    let width = details.bmWidth as u32;
    let height = details.bmHeight as u32;
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: details.bmWidth,
            biHeight: -details.bmHeight,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let dc = GetDC(None);
    if dc.is_invalid() {
        return Err("无法读取 Shell 图标设备上下文".to_string());
    }
    let lines = GetDIBits(
        dc,
        bitmap,
        0,
        height,
        Some(pixels.as_mut_ptr().cast()),
        &mut info,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, dc);
    if lines != height as i32 {
        return Err("无法读取 Shell 图标像素".to_string());
    }
    // Shell supplies premultiplied BGRA; PNG expects straight RGBA.
    let has_alpha = pixels.chunks_exact(4).any(|pixel| pixel[3] != 0);
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        if !has_alpha {
            pixel[3] = 255;
        } else if pixel[3] != 0 {
            let alpha = pixel[3] as u32;
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "Shell 图标像素无效".to_string())?;
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(STANDARD.encode(png.into_inner()))
}

unsafe fn read_registered_arguments(store: &IPropertyStore) -> Option<String> {
    let mut key = PROPERTYKEY::default();
    PSGetPropertyKeyFromName(w!("System.Link.Arguments"), &mut key).ok()?;
    let mut value = store.GetValue(&key).ok()?;
    // VT_EMPTY and VT_NULL are absent values, not confirmed empty strings.
    let present = matches!(value.as_raw().Anonymous.Anonymous.vt, 8 | 30 | 31);
    let mut buffer = vec![0u16; 32768];
    let result = if present && PropVariantToString(&value, &mut buffer).is_ok() {
        Some(read_wide_buffer(&buffer))
    } else {
        None
    };
    let _ = PropVariantClear(&mut value);
    result
}

pub fn get_all_app() -> Result<Vec<RegisteredApp>, String> {
    unsafe {
        CoInitialize(None).ok().map_err(|error| {
            format!("failed to initialize COM for AppsFolder enumeration: {error}")
        })?;
        let result = enumerate_apps_folder()
            .map_err(|error| format!("failed to enumerate shell:AppsFolder: {error}"));
        CoUninitialize();
        result
    }
}

unsafe fn enumerate_apps_folder() -> windows::core::Result<Vec<RegisteredApp>> {
    let bind_context = CreateBindCtx(0)?;
    let folder: IShellItem = SHCreateItemFromParsingName(w!("shell:AppsFolder"), &bind_context)?;
    let items: IEnumShellItems = folder.BindToHandler(&bind_context, &BHID_EnumItems)?;
    let mut apps = Vec::new();

    loop {
        let mut fetched = 0;
        let mut next = [None];
        if items.Next(&mut next, Some(&mut fetched)).is_err() || fetched == 0 {
            break;
        }
        let Some(item) = next[0].take() else { continue };
        let store: IPropertyStore = match item.BindToHandler(&bind_context, &BHID_PropertyStore) {
            Ok(store) => store,
            Err(error) => {
                debug!("failed to read AppsFolder property store: {error}");
                continue;
            }
        };
        let count = match store.GetCount() {
            Ok(count) => count,
            Err(error) => {
                debug!("failed to read AppsFolder property count: {error}");
                continue;
            }
        };

        let mut app = RegisteredApp::default();
        app.arguments = read_registered_arguments(&store);
        let mut package_path = String::new();
        let mut icon_path = String::new();
        for index in 0..count {
            let mut key = PROPERTYKEY::default();
            if store.GetAt(index, &mut key).is_err() {
                continue;
            }
            let Ok(property_name) = PSGetNameFromPropertyKey(&key) else {
                continue;
            };
            let name = String::from_utf16_lossy(property_name.as_wide());
            let Ok(mut value) = store.GetValue(&key) else {
                continue;
            };
            let mut buffer = [0u16; 2048];
            let text = if PropVariantToString(&value, &mut buffer).is_ok() {
                let end = buffer
                    .iter()
                    .position(|value| *value == 0)
                    .unwrap_or(buffer.len());
                String::from_utf16_lossy(&buffer[..end])
            } else {
                String::new()
            };
            let _ = PropVariantClear(&mut value);

            match name.as_str() {
                "System.ItemNameDisplay" => app.name = text,
                "System.AppUserModel.ID" if !text.is_empty() => {
                    app.start = format!(r"shell:AppsFolder\{text}")
                }
                "System.Link.TargetParsingPath" => app.executable = text,
                "System.AppUserModel.PackageInstallPath" => package_path = text,
                "System.Tile.SmallLogoPath" => icon_path = text,
                "System.Tile.Square150x150LogoPath" if icon_path.is_empty() => icon_path = text,
                _ => {}
            }
        }

        if app.start.is_empty() && !app.executable.is_empty() {
            app.start = app.executable.clone();
        }
        app.name = normalize_registered_app_title(&app.name);
        if app.name.is_empty() || app.start.trim().is_empty() {
            continue;
        }
        let auxiliary_target = if app.executable.is_empty() {
            app.start.as_str()
        } else {
            app.executable.as_str()
        };
        if !is_launchable_registered_target(&app.executable) {
            println!(
                "[AppIndexTrace] 跳过 AppsFolder 项（目标类型不支持）: title={}, target={}",
                app.name, app.executable
            );
            continue;
        }
        if is_auxiliary_registered_app(&app.name, auxiliary_target) {
            println!(
                "[AppIndexTrace] 跳过 AppsFolder 项（标题或目标命中辅助入口）: title={}, target={}",
                app.name, auxiliary_target
            );
            continue;
        }
        if !package_path.is_empty() && !icon_path.is_empty() {
            app.icon = resolve_package_icon(&Path::new(&package_path).join(icon_path));
        }
        apps.push(app);
    }
    Ok(apps)
}

fn resolve_package_icon(requested: &Path) -> String {
    if requested.is_file() {
        return requested.to_string_lossy().into_owned();
    }
    let (Some(parent), Some(stem)) = (
        requested.parent(),
        requested.file_stem().and_then(|value| value.to_str()),
    ) else {
        return String::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return String::new();
    };
    let stem = stem.to_ascii_lowercase();
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .map(|name| name.to_ascii_lowercase().starts_with(&stem))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    });
    candidates
        .pop()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(super) fn normalize_registered_app_title(title: &str) -> String {
    let title = title.trim();
    const LAUNCH_SUFFIXES: &[&str] = &["exe", "com", "msc", "bat", "cmd", "lnk", "rdp"];
    let path = Path::new(title);
    let should_strip = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            LAUNCH_SUFFIXES
                .iter()
                .any(|suffix| extension.eq_ignore_ascii_case(suffix))
        })
        .unwrap_or(false);
    if should_strip {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(title)
            .trim()
            .to_string()
    } else {
        title.to_string()
    }
}

fn is_launchable_registered_target(target: &str) -> bool {
    let target = target.trim().trim_matches('"');
    if target.is_empty() {
        return true;
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        return false;
    }
    if lower.starts_with("shell:") || lower.starts_with("::{") {
        return true;
    }
    Path::new(target)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "exe" | "com" | "msc" | "bat" | "cmd" | "rdp"
            )
        })
        .unwrap_or(false)
}

fn is_auxiliary_registered_app(name: &str, target: &str) -> bool {
    const AUXILIARY_TOKENS: &[&str] = &[
        "uninstall",
        "uninstaller",
        "updater",
        "update",
        "upgrade",
        "helper",
        "crashpad",
        "crashhandler",
        "readme",
        "documentation",
        "manual",
        "help",
        "installer",
        "install",
        "website",
        "release",
        "notes",
        "检查更新",
        "使用说明",
        "帮助",
        "意见反馈",
        "打字入门",
        "皮肤下载",
        "细胞词库",
        "更新",
        "卸载",
    ];
    let target_name = Path::new(target)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let name_lower = name.to_ascii_lowercase();
    let target_lower = target.trim().to_ascii_lowercase();
    let system_settings =
        target_lower.starts_with(r"shell:appsfolder\windows.immersivecontrolpanel");
    let msconfig = Path::new(target)
        .file_stem()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("msconfig"));
    if name_lower.contains("网站")
        || name_lower.contains("主页")
        || (name_lower.contains("设置") && !system_settings)
        || (name_lower.contains("配置") && !msconfig)
        || name_lower.contains("卸载")
        || name_lower.contains("解除安装")
        || name_lower.contains("检查更新")
        || name_lower.contains("使用说明")
        || name_lower.contains("帮助")
        || name_lower.contains("意见反馈")
        || name_lower.contains("打字入门")
        || name_lower.contains("皮肤下载")
        || name_lower.contains("细胞词库")
    {
        return true;
    }
    [name, target_name].iter().any(|value| {
        let lower = value.to_ascii_lowercase();
        if lower.contains("unins") || lower.contains("crashpad_handler") {
            return true;
        }
        lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|token| !token.is_empty())
            .any(|token| AUXILIARY_TOKENS.contains(&token))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        is_advertised_shortcut, is_auxiliary_registered_app, is_launchable_registered_target,
        normalize_registered_app_title,
    };

    #[test]
    fn registered_arguments_distinguish_absent_and_confirmed_empty_values() {
        use super::*;
        use windows::Win32::UI::Shell::PropertiesSystem::PSCreateMemoryPropertyStore;
        unsafe {
            CoInitialize(None).ok().unwrap();
            {
                let mut raw = std::ptr::null_mut();
                PSCreateMemoryPropertyStore(&IPropertyStore::IID, &mut raw).unwrap();
                let store = IPropertyStore::from_raw(raw);
                assert_eq!(read_registered_arguments(&store), None);
                let mut key = PROPERTYKEY::default();
                PSGetPropertyKeyFromName(w!("System.Link.Arguments"), &mut key).unwrap();
                store
                    .SetValue(&key, &windows::core::PROPVARIANT::from(""))
                    .unwrap();
                assert_eq!(read_registered_arguments(&store), Some(String::new()));
                store
                    .SetValue(&key, &windows::core::PROPVARIANT::from(r#"/K "a b.cmd""#))
                    .unwrap();
                assert_eq!(
                    read_registered_arguments(&store).as_deref(),
                    Some(r#"/K "a b.cmd""#)
                );
            }
            CoUninitialize();
        }
    }

    #[test]
    fn native_shortcut_reader_preserves_arguments_without_a_working_directory() {
        use super::*;
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lark-shortcut-{unique}.lnk"));
        let arguments = r#"/K "folder with spaces\初始化.cmd" & echo Test"#;
        unsafe {
            CoInitialize(None).ok().unwrap();
            {
                let link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
                link.SetPath(w!("%windir%\\System32\\cmd.exe")).unwrap();
                let wide: Vec<u16> = arguments.encode_utf16().chain(Some(0)).collect();
                link.SetArguments(PCWSTR(wide.as_ptr())).unwrap();
                let file: IPersistFile = link.cast().unwrap();
                let wide: Vec<u16> = path
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                file.Save(PCWSTR(wide.as_ptr()), true).unwrap();
            }
            CoUninitialize();
        }
        let result = read_shortcut(&path.to_string_lossy());
        std::fs::remove_file(&path).unwrap();
        let (target, actual) = result.unwrap();
        assert!(target.to_ascii_lowercase().ends_with(r"system32\cmd.exe"));
        assert_eq!(actual.as_deref(), Some(arguments));
    }

    #[test]
    fn native_shortcut_reader_rejects_broken_files() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lark-broken-shortcut-{unique}.lnk"));
        std::fs::write(&path, b"not a shell link").unwrap();
        let result = super::read_shortcut(&path.to_string_lossy());
        std::fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn native_shortcut_reader_reads_virtual_shell_target_from_pidl() {
        use super::*;
        let target = "::{2559A1F3-21D7-11D4-BDAF-00C04F60B9F0}";
        let path = std::env::temp_dir().join(format!(
            "lark-shell-shortcut-{}.lnk",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        unsafe {
            CoInitialize(None).ok().unwrap();
            {
                // Run.lnk stores this root PIDL; Windows cannot recreate it
                // with SHParseDisplayName from the bare Run GUID.
                let pidl: [u8; 22] = [
                    0x14, 0x00, 0x1F, 0x80, 0xF3, 0xA1, 0x59, 0x25, 0xD7, 0x21, 0xD4, 0x11, 0xBD,
                    0xAF, 0x00, 0xC0, 0x4F, 0x60, 0xB9, 0xF0, 0x00, 0x00,
                ];
                let link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
                link.SetIDList(pidl.as_ptr().cast()).unwrap();
                let file: IPersistFile = link.cast().unwrap();
                let wide: Vec<u16> = path
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                file.Save(PCWSTR(wide.as_ptr()), true).unwrap();
            }
            CoUninitialize();
        }
        let result = read_shortcut(&path.to_string_lossy());
        std::fs::remove_file(path).unwrap();
        let (actual, arguments) = result.unwrap();
        assert_eq!(actual.to_ascii_lowercase(), target.to_ascii_lowercase());
        assert_eq!(arguments, Some(String::new()));
    }

    #[test]
    fn virtual_system_entries_have_shell_icons() {
        use base64::Engine as _;
        for target in [
            "::{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}",
            r"shell:AppsFolder\Microsoft.Windows.Shell.RunDialog",
            r"shell:AppsFolder\Microsoft.Windows.ControlPanel",
            r"shell:AppsFolder\Microsoft.Windows.Explorer",
        ] {
            let icon = super::read_shell_icon_base64(target)
                .unwrap_or_else(|error| panic!("{target}: {error}"));
            let png = base64::engine::general_purpose::STANDARD
                .decode(icon)
                .unwrap();
            let image = image::load_from_memory(&png).unwrap().to_rgba8();
            assert!(image.pixels().any(|pixel| pixel[3] != 0), "{target}");
            let first = image.get_pixel(0, 0);
            assert!(
                image.pixels().any(|pixel| pixel != first),
                "Shell returned a blank icon for {target}"
            );
        }
    }

    #[test]
    fn advertised_shortcut_detection_requires_a_darwin_data_block() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let advertised = std::env::temp_dir().join(format!("lark-advertised-{unique}.lnk"));
        let ordinary = std::env::temp_dir().join(format!("lark-ordinary-{unique}.lnk"));
        let mut block = vec![0u8; 0x314];
        block[..4].copy_from_slice(&(0x314u32).to_le_bytes());
        block[4..8].copy_from_slice(&0xA0000006u32.to_le_bytes());
        std::fs::write(&advertised, block).unwrap();
        std::fs::write(&ordinary, [0u8; 16]).unwrap();
        assert!(is_advertised_shortcut(&advertised.to_string_lossy()));
        assert!(!is_advertised_shortcut(&ordinary.to_string_lossy()));
        std::fs::remove_file(advertised).unwrap();
        std::fs::remove_file(ordinary).unwrap();
    }

    #[test]
    fn registered_app_titles_strip_only_launch_suffixes() {
        assert_eq!(normalize_registered_app_title("Tool.exe"), "Tool");
        assert_eq!(normalize_registered_app_title("Console.MSC"), "Console");
        assert_eq!(normalize_registered_app_title("Shortcut.lnk"), "Shortcut");
        assert_eq!(normalize_registered_app_title("Node.js"), "Node.js");
        assert_eq!(normalize_registered_app_title("7-Zip"), "7-Zip");
    }

    #[test]
    fn registered_targets_reject_documents_and_bookmarks() {
        assert!(is_launchable_registered_target(""));
        assert!(is_launchable_registered_target(r"C:\Apps\Tool.exe"));
        assert!(is_launchable_registered_target(
            r"C:\Windows\System32\services.msc"
        ));
        assert!(!is_launchable_registered_target(r"C:\Apps\ReadMe.txt"));
        assert!(!is_launchable_registered_target(r"C:\Apps\Project.url"));
        assert!(!is_launchable_registered_target(r"C:\Apps\Guide.pdf"));
        assert!(!is_launchable_registered_target(
            "http://support.steampowered.com"
        ));
        assert!(!is_launchable_registered_target(
            "mailto:support@example.com"
        ));
        assert!(is_launchable_registered_target(
            r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"
        ));
    }

    #[test]
    fn registered_apps_reject_common_auxiliary_entries() {
        assert!(is_auxiliary_registered_app(
            "ReadMe 书签",
            r"C:\Apps\ReadMe.exe"
        ));
        assert!(is_auxiliary_registered_app(
            "Product Helper",
            r"C:\Apps\helper.exe"
        ));
        assert!(is_auxiliary_registered_app(
            "Product",
            r"C:\Apps\crashpad_handler.exe"
        ));
        assert!(!is_auxiliary_registered_app(
            "GitHub Desktop",
            r"C:\Apps\GitHubDesktop.exe"
        ));
        assert!(!is_auxiliary_registered_app("Calculator", ""));
        assert!(is_auxiliary_registered_app(
            "WeGame卸载",
            r"E:\Games\WeGame\uninstall_complete.exe"
        ));
        assert!(is_auxiliary_registered_app(
            "WeGame主页",
            r"E:\Games\WeGame\WeGame.exe"
        ));
        assert!(!is_auxiliary_registered_app(
            "Windows 设置",
            r"shell:AppsFolder\windows.immersivecontrolpanel"
        ));
        assert!(!is_auxiliary_registered_app(
            "系统配置",
            r"C:\Windows\System32\msconfig.exe"
        ));
    }

    #[test]
    fn registered_apps_reject_installers_help_and_update_entries() {
        for (name, target) in [
            ("Visual Studio Installer", r"C:\Program Files\VS\setup.exe"),
            (
                "Application Verifier Help",
                r"C:\Windows\System32\appverif.chm",
            ),
            ("检查更新", r"C:\Program Files\CheckForUpdates.exe"),
        ] {
            assert!(is_auxiliary_registered_app(name, target));
        }
    }
}
