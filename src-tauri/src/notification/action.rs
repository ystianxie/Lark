use std::path::{Path, PathBuf};

use super::model::NotificationAction;

fn normalized_target(action: &NotificationAction) -> Result<Option<(PathBuf, bool)>, String> {
    let (path, expect_dir) = match action {
        NotificationAction::Open { path, .. } => (Some(path.as_str()), false),
        NotificationAction::OpenDir { path, .. } => (Some(path.as_str()), true),
        NotificationAction::Close { .. } => (None, false),
    };
    let Some(path) = path else {
        return Ok(None);
    };
    if path.trim().is_empty() {
        return Err("通知操作路径不能为空".into());
    }
    let target = Path::new(path)
        .canonicalize()
        .map_err(|error| format!("通知操作目标不存在或不可访问：{error}"))?;
    let metadata = target
        .metadata()
        .map_err(|error| format!("无法读取通知操作目标：{error}"))?;
    if expect_dir && !metadata.is_dir() {
        return Err("打开目录操作的目标不是目录".into());
    }
    if !expect_dir && !metadata.is_file() {
        return Err("打开文件操作的目标不是文件".into());
    }
    Ok(Some((target, expect_dir)))
}

pub fn validate_action(action: &NotificationAction) -> Result<(), String> {
    normalized_target(action).map(|_| ())
}

pub fn execute_action(action: &NotificationAction) -> Result<(), String> {
    let Some((target, is_dir)) = normalized_target(action)? else {
        return Ok(());
    };
    let path = target.to_string_lossy();
    if is_dir {
        crate::api::explorer::open_explorer_result(&path)
    } else {
        crate::api::shell::open_file_result(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_action_needs_no_path() {
        assert!(validate_action(&NotificationAction::Close { label: None }).is_ok());
    }

    #[test]
    fn validates_file_and_directory_targets_separately() {
        let file = std::env::current_exe().unwrap();
        let directory = std::env::temp_dir();
        assert!(validate_action(&NotificationAction::Open {
            path: file.to_string_lossy().into_owned(),
            label: None,
        })
        .is_ok());
        assert!(validate_action(&NotificationAction::OpenDir {
            path: directory.to_string_lossy().into_owned(),
            label: None,
        })
        .is_ok());
        assert!(validate_action(&NotificationAction::Open {
            path: directory.to_string_lossy().into_owned(),
            label: None,
        })
        .is_err());
        assert!(validate_action(&NotificationAction::OpenDir {
            path: file.to_string_lossy().into_owned(),
            label: None,
        })
        .is_err());
    }

    #[test]
    fn rejects_empty_and_missing_paths_with_readable_errors() {
        let empty = NotificationAction::Open {
            path: "  ".into(),
            label: None,
        };
        assert_eq!(validate_action(&empty).unwrap_err(), "通知操作路径不能为空");
        let missing = NotificationAction::OpenDir {
            path: std::env::temp_dir()
                .join("lark-notification-target-that-does-not-exist")
                .to_string_lossy()
                .into_owned(),
            label: None,
        };
        assert!(validate_action(&missing)
            .unwrap_err()
            .contains("通知操作目标不存在或不可访问"));
    }
}
