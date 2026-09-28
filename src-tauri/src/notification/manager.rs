use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use super::model::{Notification, NotificationInput, NotificationLevel};

const MAX_NOTIFICATIONS: usize = 20;
static NEXT_NOTIFICATION_ID: AtomicU64 = AtomicU64::new(1);

/// Thread-safe access is provided by managing this value as `Mutex<NotificationManager>`.
pub struct NotificationManager {
    notifications: Vec<Notification>,
    enabled: bool,
    paused: bool,
    deadlines: HashMap<String, Instant>,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self {
            notifications: Vec::new(),
            enabled: true,
            paused: false,
            deadlines: HashMap::new(),
        }
    }
}

impl NotificationManager {
    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.clone()
    }

    pub fn enqueue(&mut self, input: NotificationInput) -> Option<Notification> {
        if !self.enabled
            || (self.paused
                && matches!(
                    input.level,
                    NotificationLevel::Success | NotificationLevel::Info
                ))
        {
            return None;
        }

        let duration_ms = if input.level == NotificationLevel::Error {
            None
        } else {
            input
                .duration_ms
                .or_else(|| input.level.default_duration_ms())
        };
        let notification = Notification {
            id: format!(
                "notification-{}-{}",
                now_ms(),
                NEXT_NOTIFICATION_ID.fetch_add(1, Ordering::Relaxed)
            ),
            level: input.level,
            title: input.title,
            message: input.message,
            actions: input.actions,
            duration_ms: duration_ms.filter(|duration| *duration > 0),
            created_at_ms: now_ms(),
        };
        if let Some(duration_ms) = notification.duration_ms {
            self.deadlines.insert(
                notification.id.clone(),
                Instant::now() + Duration::from_millis(duration_ms),
            );
        }

        // Newest first. At capacity, prefer evicting the oldest non-critical item.
        if self.notifications.len() >= MAX_NOTIFICATIONS {
            let victim = self
                .notifications
                .iter()
                .rposition(|item| {
                    matches!(
                        item.level,
                        NotificationLevel::Success | NotificationLevel::Info
                    )
                })
                .unwrap_or(self.notifications.len() - 1);
            let evicted = self.notifications.remove(victim);
            self.deadlines.remove(&evicted.id);
        }
        self.notifications.insert(0, notification.clone());
        Some(notification)
    }

    pub fn dismiss(&mut self, id: &str) -> bool {
        let before = self.notifications.len();
        self.notifications.retain(|item| item.id != id);
        self.deadlines.remove(id);
        self.notifications.len() != before
    }

    pub fn expire_due(&mut self) -> bool {
        self.expire_due_at(Instant::now())
    }

    fn expire_due_at(&mut self, now: Instant) -> bool {
        let expired: Vec<String> = self
            .deadlines
            .iter()
            .filter(|(id, deadline)| {
                **deadline <= now
                    && !(self.paused
                        && self.notifications.iter().any(|item| {
                            item.id == **id && item.level == NotificationLevel::Warning
                        }))
            })
            .map(|(id, _)| id.clone())
            .collect();
        if expired.is_empty() {
            return false;
        }
        self.notifications
            .retain(|item| !expired.iter().any(|id| id == &item.id));
        for id in expired {
            self.deadlines.remove(&id);
        }
        true
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn reset_warning_durations(&mut self) {
        for notification in &mut self.notifications {
            if notification.level == NotificationLevel::Warning {
                let duration_ms = notification.duration_ms.unwrap_or(8_000);
                notification.created_at_ms = now_ms();
                self.deadlines.insert(
                    notification.id.clone(),
                    Instant::now() + Duration::from_millis(duration_ms),
                );
            }
        }
    }
}

/// Start the single expiration loop for the app. The UI timer provides smooth
/// feedback; Rust remains authoritative and expires notifications if the view
/// is hidden, reloaded, or not listening to events.
pub fn start_expiration_worker(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(100));
        let Some(state) = app.try_state::<std::sync::Mutex<NotificationManager>>() else {
            break;
        };
        let changed = state.lock().unwrap().expire_due();
        if changed {
            let manager = state.lock().unwrap();
            let payload = serde_json::json!({
                "notifications": manager.notifications(),
                "enabled": manager.is_enabled(),
                "paused": manager.is_paused(),
            });
            let should_hide = manager.notifications().is_empty();
            drop(manager);
            let _ = app.emit("notification-state", payload);
            if should_hide {
                if let Some(window) = app.get_webview_window("notification") {
                    let _ = window.hide();
                }
            }
        }
    });
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(level: NotificationLevel, title: &str) -> NotificationInput {
        NotificationInput {
            level,
            title: title.into(),
            message: None,
            actions: vec![],
            duration_ms: None,
        }
    }

    #[test]
    fn inserts_newest_first_and_caps_queue_at_twenty() {
        let mut manager = NotificationManager::default();
        for index in 0..21 {
            manager.enqueue(input(NotificationLevel::Info, &index.to_string()));
        }
        let items = manager.notifications();
        assert_eq!(items.len(), MAX_NOTIFICATIONS);
        assert_eq!(items[0].title, "20");
        assert_eq!(items.last().unwrap().title, "1");
    }

    #[test]
    fn paused_manager_drops_info_but_keeps_warning_and_error() {
        let mut manager = NotificationManager::default();
        manager.set_paused(true);
        assert!(manager
            .enqueue(input(NotificationLevel::Info, "drop"))
            .is_none());
        assert!(manager
            .enqueue(input(NotificationLevel::Warning, "keep"))
            .is_some());
        assert_eq!(manager.notifications().len(), 1);
    }

    #[test]
    fn disabled_manager_rejects_all_notifications() {
        let mut manager = NotificationManager::default();
        manager.set_enabled(false);
        assert!(manager
            .enqueue(input(NotificationLevel::Error, "drop"))
            .is_none());
    }

    #[test]
    fn resume_restarts_warning_duration_from_full_default() {
        let mut manager = NotificationManager::default();
        manager.set_paused(true);
        manager.enqueue(input(NotificationLevel::Warning, "kept"));
        let before = manager.notifications()[0].created_at_ms;
        manager.reset_warning_durations();
        let notification = manager.notifications()[0].clone();
        assert_eq!(notification.duration_ms, Some(8_000));
        assert!(notification.created_at_ms >= before);
    }

    #[test]
    fn resume_preserves_custom_warning_duration_and_is_independent_of_enabled() {
        let mut manager = NotificationManager::default();
        let mut warning = input(NotificationLevel::Warning, "custom");
        warning.duration_ms = Some(12_000);
        manager.enqueue(warning);
        let id = manager.notifications()[0].id.clone();

        manager.set_paused(true);
        manager.set_enabled(false);
        assert!(manager.is_paused());
        assert!(!manager.is_enabled());
        manager.set_paused(false);
        manager.reset_warning_durations();

        assert!(!manager.is_paused());
        assert!(!manager.is_enabled());
        assert_eq!(manager.notifications()[0].duration_ms, Some(12_000));
        assert!(manager.deadlines[&id] > Instant::now() + Duration::from_secs(11));
        assert!(manager
            .enqueue(input(NotificationLevel::Error, "still disabled"))
            .is_none());
    }

    #[test]
    fn pause_drops_success_and_info_but_retains_warning_and_error() {
        let mut manager = NotificationManager::default();
        manager.set_paused(true);
        assert!(manager
            .enqueue(input(NotificationLevel::Success, "drop success"))
            .is_none());
        assert!(manager
            .enqueue(input(NotificationLevel::Info, "drop info"))
            .is_none());
        assert!(manager
            .enqueue(input(NotificationLevel::Warning, "keep warning"))
            .is_some());
        assert!(manager
            .enqueue(input(NotificationLevel::Error, "keep error"))
            .is_some());
        assert_eq!(manager.notifications().len(), 2);
    }

    #[test]
    fn expiration_removes_due_items_and_manual_dismiss_clears_deadline() {
        let mut manager = NotificationManager::default();
        manager.enqueue(input(NotificationLevel::Success, "expires"));
        let expired =
            manager.deadlines.values().next().copied().unwrap() + Duration::from_millis(1);
        assert!(manager.expire_due_at(expired));
        assert!(manager.notifications().is_empty());

        manager.enqueue(input(NotificationLevel::Warning, "dismissed"));
        let id = manager.notifications()[0].id.clone();
        assert!(manager.dismiss(&id));
        assert!(!manager.deadlines.contains_key(&id));
    }

    #[test]
    fn warning_deadline_is_preserved_during_pause_then_restarted() {
        let mut manager = NotificationManager::default();
        manager.enqueue(input(NotificationLevel::Warning, "warning"));
        let id = manager.notifications()[0].id.clone();
        let old_deadline = manager.deadlines[&id];
        manager.set_paused(true);
        assert!(!manager.expire_due_at(old_deadline + Duration::from_secs(1)));
        assert_eq!(manager.notifications().len(), 1);
        manager.reset_warning_durations();
        assert!(manager.deadlines[&id] > old_deadline);
    }

    #[test]
    fn generated_ids_are_unique_even_for_same_millisecond() {
        let mut manager = NotificationManager::default();
        let first = manager
            .enqueue(input(NotificationLevel::Info, "1"))
            .unwrap();
        let second = manager
            .enqueue(input(NotificationLevel::Info, "2"))
            .unwrap();
        assert_ne!(first.id, second.id);
    }
}
