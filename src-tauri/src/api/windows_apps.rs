use std::{
    fs,
    path::{Path, PathBuf},
};

use log::{debug, error};
use windows::{
    core::{w, Interface},
    Win32::{
        System::Com::{
            CoInitialize, CoUninitialize, CreateBindCtx,
            StructuredStorage::{PropVariantClear, PropVariantToString},
        },
        UI::Shell::{
            BHID_EnumItems, BHID_PropertyStore, IEnumShellItems, IShellItem,
            PropertiesSystem::{IPropertyStore, PSGetNameFromPropertyKey, PROPERTYKEY},
            SHCreateItemFromParsingName,
        },
    },
};

#[derive(Default)]
pub struct RegisteredApp {
    pub name: String,
    pub icon: String,
    pub start: String,
    pub executable: String,
}

pub fn get_all_app() -> Vec<RegisteredApp> {
    unsafe {
        if CoInitialize(None).is_err() {
            error!("failed to initialize COM for AppsFolder enumeration");
            return Vec::new();
        }
        let result = enumerate_apps_folder();
        CoUninitialize();
        match result {
            Ok(items) => items,
            Err(error) => {
                error!("failed to enumerate shell:AppsFolder: {error}");
                Vec::new()
            }
        }
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
        if !is_launchable_registered_target(&app.executable)
            || is_auxiliary_registered_app(&app.name, &app.executable)
        {
            debug!(
                "skip non-application AppsFolder item: name={:?}, target={:?}",
                app.name, app.executable
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

fn normalize_registered_app_title(title: &str) -> String {
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
    Path::new(target)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "exe" | "com" | "msc" | "bat" | "cmd"
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
        "website",
    ];
    let target_name = Path::new(target)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
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
        is_auxiliary_registered_app, is_launchable_registered_target,
        normalize_registered_app_title,
    };

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
    }
}
