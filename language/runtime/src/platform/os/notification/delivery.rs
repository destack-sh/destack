use crate::diagnostic::RuntimeResult;
use crate::host::{Platform, RequestContext};
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{NotificationCategoryValue, NotificationRequestValue};

#[cfg(target_os = "macos")]
use crate::host::os::macos::request::notification::backend as notification_backend;
#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
use crate::host::os::unix::request::notification::backend as notification_backend;
#[cfg(windows)]
use crate::host::os::windows::request::notification::core as notification_backend;
#[cfg(test)]
use crate::platform::os::notification::runtime::desktop_notification_test_mode_enabled;
#[cfg(not(any(
    windows,
    target_os = "macos",
    all(
        unix,
        not(any(target_os = "android", target_os = "ios", target_os = "macos"))
    )
)))]
use crate::platform::os::notification::unsupported as notification_backend;

/// Deliver one notification through the active desktop host.
pub(super) fn deliver_notification(
    context: &RequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    deliver_native_notification(context, id, request)
}

/// Cancel one delivered native notification when the platform exposes cancellation.
pub(super) fn cancel_native_notification(context: &RequestContext, id: &str) -> RuntimeResult<()> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    cancel_platform_notification(context, id)
}

/// Schedule one notification through the active desktop host.
pub(super) fn schedule_native_notification(
    context: &RequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    schedule_platform_notification(context, id, request)
}

/// Cancel one pending scheduled notification through the active desktop host.
pub(super) fn cancel_native_pending_notification(
    context: &RequestContext,
    id: &str,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
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
    if desktop_notification_test_mode_enabled() {
        return Ok(());
    }

    set_platform_notification_categories(categories)
}

/// Request one native notification permission state through the active host.
pub(super) fn request_notification_permission(
    context: &RequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        return Ok(NotificationPermissionState::Granted);
    }

    notification_backend::request_permission(context)
}

/// Deliver one notification through the compiled host backend.
fn deliver_native_notification(
    context: &RequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    notification_backend::deliver_notification(context, id, request)
}

/// Cancel one delivered notification through the compiled host backend.
fn cancel_platform_notification(context: &RequestContext, id: &str) -> RuntimeResult<()> {
    notification_backend::cancel_notification(context, id)
}

/// Schedule one notification through the compiled host backend.
fn schedule_platform_notification(
    context: &RequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    notification_backend::schedule_notification(context, id, request)
}

/// Cancel one pending scheduled notification through the compiled host backend.
fn cancel_platform_pending_notification(context: &RequestContext, id: &str) -> RuntimeResult<()> {
    notification_backend::cancel_pending_notification(context, id)
}

/// Register one notification category set through the compiled host backend.
fn set_platform_notification_categories(
    categories: &[NotificationCategoryValue],
) -> RuntimeResult<()> {
    notification_backend::set_categories(categories)
}
