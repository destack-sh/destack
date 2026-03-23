use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::host::core::{HostSessionContext, HostSessionId};
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

/// Return the default unsupported notification permission state.
pub(super) fn request_permission(
    _context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    Err(not_supported("destack.os.notification.requestPermission"))
}

/// Reject unsupported notification delivery.
pub(super) fn deliver_notification(
    _context: &HostRequestContext,
    _id: &str,
    _request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.post"))
}

/// Reject unsupported notification cancellation.
pub(super) fn cancel_notification(_context: &HostRequestContext, _id: &str) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.cancel"))
}

/// Reject unsupported notification scheduling.
pub(super) fn schedule_notification(
    _context: &HostRequestContext,
    _id: &str,
    _request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.schedule"))
}

/// Reject unsupported pending notification enumeration.
pub(super) fn list_pending_notifications(
    _context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    Err(not_supported("destack.os.notification.pendingList"))
}

/// Reject unsupported pending notification cancellation.
pub(super) fn cancel_pending_notification(
    _context: &HostRequestContext,
    _id: &str,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.pendingCancel"))
}

/// Reject unsupported bulk pending notification cancellation.
pub(super) fn cancel_all_pending_notifications() -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.pendingCancelAll"))
}

/// Reject unsupported notification category registration.
pub(super) fn set_categories(_categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.categorySet"))
}

/// Remove unsupported notification backend state for one runtime when present.
pub(super) fn unregister_runtime(_host_runtime_id: HostSessionId) {}

/// Service unsupported notification ingress.
pub(super) fn service_notification_ingress(_context: &HostSessionContext) -> RuntimeResult<()> {
    Ok(())
}
