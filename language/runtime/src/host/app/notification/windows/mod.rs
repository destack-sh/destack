mod activation;
mod core;
mod identity;
mod toast;

pub(in crate::host::app::notification) use core::{
    cancel_notification, cancel_pending_notification, deliver_notification,
    list_pending_notifications, request_permission, schedule_notification,
    service_notification_ingress, set_categories, unregister_runtime,
};
