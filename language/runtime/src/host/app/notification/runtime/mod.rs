mod category;
mod event;
mod pending;
mod permission;
mod posted;
mod state;

#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
pub(in crate::host::app::notification) use category::notification_category;
pub(in crate::host::app::notification) use category::{
    list_notification_categories, set_notification_categories,
};
pub(in crate::host::app::notification) use event::publish_delivered_notification;
#[cfg(any(target_os = "macos", windows, all(unix, not(target_os = "macos"))))]
pub(in crate::host::app::notification) use event::{
    next_notification_sequence, publish_dismissed_notification, publish_interacted_notification,
};
pub(in crate::host::app::notification) use pending::{
    cancel_all_pending_notifications, cancel_pending_notification, list_pending_notifications,
    schedule_notification,
};
pub(in crate::host::app::notification) use permission::request_notification_permission;
pub(in crate::host::app::notification) use posted::{
    cancel_all_notifications, cancel_notification, post_notification, remove_posted_notification,
};
#[cfg(test)]
pub(in crate::host::app::notification) use state::desktop_notification_test_mode_enabled;
pub(in crate::host::app::notification) use state::notification_runtime_service;
#[cfg(test)]
pub(crate) use state::with_notification_test_mode;
