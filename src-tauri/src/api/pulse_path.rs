//! Pulse 1.0.53 的 Shell 条目不包含激活标签页标识。
//! 使用其 v7 会话快照中的 `path`（激活标签页/分栏），而非第一个 Shell 条目。
//! Pulse 每约 3 秒自动保存一次；本模块只读，不改变界面或剪贴板。

use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct PulseSession {
    version: u32,
    path: String,
}

fn session_path(json: &str) -> Result<Option<PathBuf>, String> {
    let session: PulseSession = serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("读取 Pulse 会话失败: {error}"))?;
    // 新版本的多窗口会话可能需要按 HWND 选择，不能猜测未知格式。
    if session.version != 7 {
        return Err(format!("暂不支持 Pulse 会话版本 {}", session.version));
    }
    if session.path.is_empty() || session.path.starts_with("pulse:") {
        return Ok(None);
    }
    let path = match session.path.strip_prefix(r"\\?\") {
        Some(rest) => match rest.strip_prefix("UNC\\") {
            Some(unc) => format!(r"\\{unc}"),
            None => rest.to_string(),
        },
        None => session.path,
    };
    // 只接受盘符绝对路径和 UNC 路径，避免把虚拟页/相对路径作为目录。
    let bytes = path.as_bytes();
    let drive_path = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    if !drive_path && !path.starts_with(r"\\") {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(path)))
}

#[cfg(target_os = "windows")]
pub fn active_path() -> Result<Option<PathBuf>, String> {
    let local_data =
        std::env::var_os("LOCALAPPDATA").ok_or_else(|| "无法定位 Pulse 会话目录".to_string())?;
    let file = PathBuf::from(local_data).join("Pulse").join("session.json");
    let json = std::fs::read_to_string(file)
        .map_err(|error| format!("读取 Pulse 会话文件失败: {error}"))?;
    Ok(session_path(&json)?.filter(|path| path.is_dir()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_active_path_instead_of_first_tab() {
        let json = serde_json::json!({
            "version": 7,
            "path": r"\\?\E:\Project",
            "activeTab": 1,
            "layoutTabs": [
                {"panes": [{"path": r"D:\first"}]},
                {"panes": [{"path": r"E:\Project"}]}
            ]
        });
        assert_eq!(
            session_path(&json.to_string()).unwrap(),
            Some(PathBuf::from(r"E:\Project"))
        );
    }

    #[test]
    fn uses_focused_pane_path_and_preserves_unc() {
        let json = serde_json::json!({"version": 7, "path": r"\\?\UNC\server\share\中文 目录", "activeTab": 0});
        assert_eq!(
            session_path(&json.to_string()).unwrap(),
            Some(PathBuf::from(r"\\server\share\中文 目录"))
        );
    }

    #[test]
    fn rejects_virtual_pages_relative_paths_and_unknown_versions() {
        for path in ["", "pulse:search:hello", "relative", "C:relative"] {
            let json = serde_json::json!({"version": 7, "path": path});
            assert_eq!(session_path(&json.to_string()).unwrap(), None);
        }
        assert!(session_path(r#"{"version":8,"path":"C:\\old"}"#).is_err());
        assert!(session_path("{").is_err());
    }
}
