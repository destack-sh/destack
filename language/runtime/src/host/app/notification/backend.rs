use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequestContext, HostRuntimeId};
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

#[cfg(target_os = "linux")]
use super::linux as notification_backend;
#[cfg(target_os = "macos")]
use super::macos as notification_backend;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
use super::unix as notification_backend;
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    all(unix, not(any(target_os = "linux", target_os = "macos"))),
    windows,
)))]
use super::unsupported as notification_backend;
#[cfg(windows)]
use super::windows as notification_backend;

/// Request one native notification permission state through the active backend.
pub(super) fn request_notification_permission(
    context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    notification_backend::request_permission(context)
}

/// Deliver one notification through the active backend.
pub(super) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    notification_backend::deliver_notification(context, id, request)
}

/// Cancel one delivered notification through the active backend.
pub(super) fn cancel_notification(context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    notification_backend::cancel_notification(context, id)
}

/// Schedule one notification through the active backend.
pub(super) fn schedule_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    notification_backend::schedule_notification(context, id, request)
}

/// Return pending scheduled notifications through the active backend.
pub(super) fn list_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    notification_backend::list_pending_notifications(context)
}

/// Cancel one pending scheduled notification through the active backend.
pub(super) fn cancel_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    notification_backend::cancel_pending_notification(context, id)
}

/// Register one notification category set through the active backend.
pub(super) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    notification_backend::set_categories(categories)
}

/// Remove backend notification state for one runtime when needed.
pub(super) fn unregister_runtime(host_runtime_id: HostRuntimeId) {
    notification_backend::unregister_runtime(host_runtime_id);
}

/// Service backend-owned notification ingress for one runtime.
pub(super) fn service_notification_ingress(context: &HostRequestContext) -> RuntimeResult<()> {
    notification_backend::service_notification_ingress(context)
}
