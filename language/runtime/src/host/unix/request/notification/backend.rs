use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequestContext, HostSessionContext, HostSessionId};
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

#[cfg(all(unix, not(target_vendor = "apple"), not(target_os = "linux")))]
use super::core as notification_backend;
#[cfg(target_os = "linux")]
use super::linux as notification_backend;

/// Request one notification permission state through the active Unix backend.
pub(crate) fn request_permission(
    context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    notification_backend::request_permission(context)
}

/// Deliver one notification through the active Unix backend.
pub(crate) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    notification_backend::deliver_notification(context, id, request)
}

/// Cancel one delivered notification through the active Unix backend.
pub(crate) fn cancel_notification(context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    notification_backend::cancel_notification(context, id)
}

/// Schedule one notification through the active Unix backend.
pub(crate) fn schedule_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    notification_backend::schedule_notification(context, id, request)
}

/// List pending notifications through the active Unix backend.
pub(crate) fn list_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    notification_backend::list_pending_notifications(context)
}

/// Cancel one pending notification through the active Unix backend.
pub(crate) fn cancel_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    notification_backend::cancel_pending_notification(context, id)
}

/// Register notification categories through the active Unix backend.
pub(crate) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    notification_backend::set_categories(categories)
}

/// Remove one runtime from the active Unix notification backend.
pub(crate) fn unregister_runtime(host_session_id: HostSessionId) {
    notification_backend::unregister_runtime(host_session_id);
}

/// Service notification ingress through the active Unix backend.
pub(crate) fn service_notification_ingress(context: &HostSessionContext) -> RuntimeResult<()> {
    notification_backend::service_notification_ingress(context)
}
