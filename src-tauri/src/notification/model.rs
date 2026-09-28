use serde::{Deserialize, Serialize};

/// Visual priority of an in-app notification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Success,
    Info,
    Warning,
    Error,
}

impl NotificationLevel {
    pub fn default_duration_ms(self) -> Option<u64> {
        match self {
            Self::Success | Self::Info => Some(5_000),
            Self::Warning => Some(8_000),
            Self::Error => None,
        }
    }
}

/// Actions are data-only; execution is handled by the Rust host.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationAction {
    Open { path: String, label: Option<String> },
    OpenDir { path: String, label: Option<String> },
    Close { label: Option<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub level: NotificationLevel,
    pub title: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
    /// `None` means no automatic dismissal (the default for errors).
    pub duration_ms: Option<u64>,
    /// Epoch milliseconds, assigned by the manager when enqueued.
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationInput {
    pub level: NotificationLevel,
    pub title: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
    /// A supplied duration overrides the level default. Zero disables timeout.
    pub duration_ms: Option<u64>,
}

impl NotificationInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() || self.title.chars().count() > 200 {
            return Err("通知标题必填且不能超过 200 个字符".into());
        }
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.chars().count() > 4_000)
        {
            return Err("通知内容不能超过 4000 个字符".into());
        }
        if self.actions.len() > 3 {
            return Err("单条通知最多支持 3 个操作".into());
        }
        for action in &self.actions {
            match action {
                NotificationAction::Open { path, label }
                | NotificationAction::OpenDir { path, label } => {
                    if path.trim().is_empty() || path.chars().count() > 32_767 {
                        return Err("通知操作路径无效".into());
                    }
                    validate_action_label(label.as_deref())?;
                }
                NotificationAction::Close { label } => validate_action_label(label.as_deref())?,
            }
        }
        if let Some(duration) = self.duration_ms {
            if self.level == NotificationLevel::Error {
                return Err("error 通知必须手动关闭".into());
            }
            if !(1_000..=60_000).contains(&duration) {
                return Err("通知 durationMs 必须在 1000 到 60000 毫秒之间".into());
            }
        }
        Ok(())
    }
}

fn validate_action_label(label: Option<&str>) -> Result<(), String> {
    if label.is_some_and(|value| value.chars().count() > 40) {
        return Err("通知操作 label 不能超过 40 个字符".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_json_uses_frontend_field_names() {
        let value = serde_json::to_value(Notification {
            id: "n1".into(),
            level: NotificationLevel::Warning,
            title: "Heads up".into(),
            message: Some("Detail".into()),
            actions: vec![NotificationAction::OpenDir {
                path: "C:/tmp".into(),
                label: None,
            }],
            duration_ms: Some(8_000),
            created_at_ms: 12,
        })
        .unwrap();
        assert_eq!(value["durationMs"], 8_000);
        assert_eq!(value["createdAtMs"], 12);
        assert_eq!(value["actions"][0]["type"], "open_dir");
    }

    #[test]
    fn default_durations_match_notification_levels() {
        assert_eq!(
            NotificationLevel::Success.default_duration_ms(),
            Some(5_000)
        );
        assert_eq!(NotificationLevel::Info.default_duration_ms(), Some(5_000));
        assert_eq!(
            NotificationLevel::Warning.default_duration_ms(),
            Some(8_000)
        );
        assert_eq!(NotificationLevel::Error.default_duration_ms(), None);
    }
}
