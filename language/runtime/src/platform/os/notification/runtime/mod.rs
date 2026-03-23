mod category;
mod event;
mod pending;
mod permission;
mod posted;
mod state;

#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
pub(crate) use category::notification_category;
pub(crate) use category::{list_notification_categories, set_notification_categories};
pub(crate) use event::publish_delivered_notification;
#[cfg(any(target_os = "macos", windows, all(unix, not(target_os = "macos"))))]
pub(crate) use event::{
    next_notification_sequence, publish_dismissed_notification, publish_interacted_notification,
};
pub(crate) use pending::{
    cancel_all_pending_notifications, cancel_pending_notification, list_pending_notifications,
    schedule_notification,
};
pub(crate) use permission::request_notification_permission;
pub(crate) use posted::{
    cancel_all_notifications, cancel_notification, post_notification, remove_posted_notification,
};
#[cfg(test)]
pub(crate) use state::desktop_notification_test_mode_enabled;
pub(crate) use state::notification_runtime_service;
#[cfg(test)]
pub(crate) use state::with_notification_test_mode;
