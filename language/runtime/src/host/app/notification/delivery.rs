use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::host::core::HostRequestContext;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

use super::backend;

/// Deliver one notification through the active desktop host.
pub(super) fn deliver_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    deliver_native_notification(context, id, request)
}

/// Cancel one delivered native notification when the platform exposes cancellation.
pub(super) fn cancel_native_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    cancel_platform_notification(context, id)
}

/// Schedule one notification through the active desktop host.
pub(super) fn schedule_native_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    schedule_platform_notification(context, id, request)
}

/// Return pending scheduled notifications through the active desktop host.
pub(super) fn list_native_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(Vec::new());
    }

    list_platform_pending_notifications(context)
}

/// Cancel one pending scheduled notification through the active desktop host.
pub(super) fn cancel_native_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    cancel_platform_pending_notification(context, id)
}

/// Register native notification categories when the platform exposes category APIs.
pub(super) fn set_native_notification_categories(
    _platform: Platform,
    categories: &[NotificationCategoryValue],
) -> RuntimeResult<()> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    set_platform_notification_categories(categories)
}

/// Request one native notification permission state through the active host.
pub(super) fn request_notification_permission(
    context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    #[cfg(test)]
    if super::runtime::desktop_notification_test_mode_enabled() {
        return Ok(NotificationPermissionState::Granted);
    }

    backend::request_notification_permission(context)
}

/// Deliver one notification through the compiled host backend.
fn deliver_native_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    backend::deliver_notification(context, id, request)
}

/// Cancel one delivered notification through the compiled host backend.
fn cancel_platform_notification(context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    backend::cancel_notification(context, id)
}

/// Schedule one notification through the compiled host backend.
fn schedule_platform_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    backend::schedule_notification(context, id, request)
}

/// Return pending scheduled notifications through the compiled host backend.
fn list_platform_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    backend::list_pending_notifications(context)
}

/// Cancel one pending scheduled notification through the compiled host backend.
fn cancel_platform_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    backend::cancel_pending_notification(context, id)
}

/// Register one notification category set through the compiled host backend.
fn set_platform_notification_categories(
    categories: &[NotificationCategoryValue],
) -> RuntimeResult<()> {
    backend::set_categories(categories)
}
