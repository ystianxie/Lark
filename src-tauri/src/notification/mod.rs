//! Notification domain types and in-memory manager.

pub mod action;
pub mod manager;
pub mod model;

pub use manager::NotificationManager;
pub use model::{NotificationInput, NotificationLevel};

pub fn validate_input(input: &NotificationInput) -> Result<(), String> {
    input.validate()
}
