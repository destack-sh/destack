use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRequestContext;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{NotificationCategoryValue, NotificationRequestValue};

use super::schedule;
use crate::host::app::notification::unix;

/// Return the Linux desktop notification permission state.
pub(super) fn request_permission(
    context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    unix::request_permission(context)
}

/// Deliver one notification through the Linux desktop host.
pub(super) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    unix::deliver_notification(context, id, request)
}

/// Cancel one delivered Linux notification.
pub(super) fn cancel_notification(context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    unix::cancel_notification(context, id)
}

/// Reject unsupported bulk pending notification cancellation.
pub(super) fn cancel_all_pending_notifications() -> RuntimeResult<()> {
    schedule::cancel_all_pending_notifications()
}

/// Validate Linux notification categories against the freedesktop action model.
pub(super) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    unix::set_categories(categories)
}
