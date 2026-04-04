use crate::diagnostic::RuntimeResult;
use crate::host::RequestContext;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{NotificationCategoryValue, NotificationRequestValue};

use crate::host::os::unix::request::notification::core as unix_notification;

use super::schedule;

/// Return the Linux desktop notification permission state.
pub(crate) fn request_permission(
    context: &RequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    unix_notification::request_permission(context)
}

/// Deliver one notification through the Linux desktop host.
pub(crate) fn deliver_notification(
    context: &RequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    unix_notification::deliver_notification(context, id, request)
}

/// Cancel one delivered Linux notification.
pub(super) fn cancel_notification(context: &RequestContext, id: &str) -> RuntimeResult<()> {
    unix_notification::cancel_notification(context, id)
}

/// Reject unsupported bulk pending notification cancellation.
pub(super) fn cancel_all_pending_notifications() -> RuntimeResult<()> {
    schedule::cancel_all_pending_notifications()
}

/// Validate Linux notification categories against the freedesktop action model.
pub(crate) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    unix_notification::set_categories(categories)
}
