use std::path::Path;

fn is_supported_target(path: &str) -> bool {
    !path.contains('\0')
        && Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

/// Run elevation off the UI thread: the system UAC dialog may wait for user input.
#[tauri::command(rename_all = "camelCase")]
pub async fn open_app_as_admin(app_path: String, app_arguments: String) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(move || launch_elevated(&app_path, &app_arguments))
            .await
            .map_err(|error| format!("管理员启动任务失败：{error}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app_path, app_arguments);
        Err("管理员启动目前仅支持 Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
fn launch_elevated(app_path: &str, app_arguments: &str) -> Result<bool, String> {
    use windows::{
        core::{w, PCWSTR},
        Win32::UI::{
            Shell::{ShellExecuteExW, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };

    let path = app_path.trim().trim_matches('"');
    if path.contains('\0') || app_arguments.contains('\0') {
        return Err("应用路径或参数包含无效字符".to_string());
    }
    let (target, arguments, working_directory) = if Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        let shortcut = super::windows_apps::read_shortcut_details(path)?;
        (
            shortcut.target,
            shortcut
                .arguments
                .unwrap_or_else(|| app_arguments.to_string()),
            shortcut.working_directory,
        )
    } else {
        (path.to_string(), app_arguments.to_string(), String::new())
    };
    if !is_supported_target(&target) {
        return Err("此应用不支持以管理员身份运行，仅支持桌面可执行程序及其快捷方式".to_string());
    }
    if arguments.contains('\0') || working_directory.contains('\0') {
        return Err("快捷方式包含无效字符".to_string());
    }
    let directory = if working_directory.is_empty() {
        Path::new(&target)
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        working_directory
    };
    let wide = |value: &str| value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let target = wide(&target);
    let arguments = wide(&arguments);
    let directory = wide(&directory);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(target.as_ptr()),
        lpParameters: PCWSTR(arguments.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    match unsafe { ShellExecuteExW(&mut info) } {
        Ok(()) => Ok(true),
        Err(error) if error.code().0 as u32 == 0x800704C7 => Ok(false), // ERROR_CANCELLED
        Err(error) => Err(format!("以管理员身份运行失败：{error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::is_supported_target;

    #[test]
    fn elevation_accepts_executables_and_rejects_shell_entries_and_documents() {
        assert!(is_supported_target(r"C:\Program Files\工具\APP.EXE"));
        for path in [
            "",
            "shell:AppsFolder\\App!App",
            "https://example.com",
            "file.txt",
            "file.lnk",
            "bad\0.exe",
        ] {
            assert!(!is_supported_target(path), "{path:?}");
        }
    }
}
