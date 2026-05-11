use notify_rust::{Hint, Notification, Timeout, Urgency};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::error::not_supported;
use crate::host::os::unix::identity::{resolved_application_identifier, resolved_display_name};
use crate::host::{HostSessionId, Platform, RequestContext, SessionContext};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};
use crate::platform::os::notification::runtime;
use crate::platform::os::{NotificationPermissionState, NotificationPriority};

use super::action::{
    close_notification, unix_notification_server_available, unix_notification_supports_actions,
};
use super::category::set_categories as set_unix_categories;
use super::response::{
    register_active_notification, remove_active_notification, unregister_notification_runtime,
};

/// Return the Unix desktop notification permission state.
pub(super) fn request_permission(
    _context: &RequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    if unix_notification_server_available() {
        return Ok(NotificationPermissionState::Granted);
    }

    Ok(NotificationPermissionState::Denied)
}

/// Deliver one notification through the Unix desktop host.
pub(super) fn deliver_notification(
    context: &RequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    let app_identifier = resolved_application_identifier(context)?;
    let display_name = resolved_display_name(context)?;
    let mut notification = Notification::new();

    // base content
    notification.summary(&request.title);
    notification.body(&request.body);
    notification.appname(&display_name);
    notification.id(0);
    notification.hint(Hint::DesktopEntry(app_identifier));

    if let Some(sound) = &request.sound {
        notification.sound_name(sound);
    }

    if let Some(thread_id) = &request.thread_id {
        notification.hint(Hint::Category(thread_id.clone()));
    }

    // runtime action routing
    apply_notification_actions(&mut notification, context.host_session_id, request)?;

    // urgency
    let urgency = match request.priority {
        NotificationPriority::Low => Urgency::Low,
        NotificationPriority::Normal => Urgency::Normal,
        NotificationPriority::High => Urgency::Critical,
    };

    notification.urgency(urgency);
    notification.timeout(Timeout::Default);

    let handle = notification.show().map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("destack.os.notification.post failed in the Unix notification host: {error}"),
        ))
        .boxed()
    })?;
    let server_id = handle.id();

    // register the host notification route before response signals arrive
    register_active_notification(
        context.host_session_id,
        context.platform,
        id.to_string(),
        request.clone(),
        server_id,
    )?;

    Ok(())
}

/// Cancel one delivered Unix notification by its host identifier when present.
pub(super) fn cancel_notification(context: &RequestContext, id: &str) -> RuntimeResult<()> {
    let Some(server_id) = remove_active_notification(context.host_session_id, id)? else {
        return Ok(());
    };

    close_notification(server_id)
}

/// Reject Unix notification scheduling until it is backed by a real host scheduler.
pub(super) fn schedule_notification(
    _context: &RequestContext,
    _id: &str,
    _request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.schedule"))
}

/// Reject Unix pending notification enumeration until it is backed by a real host scheduler.
pub(super) fn list_pending_notifications(
    _context: &RequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    Err(not_supported("destack.os.notification.pendingList"))
}

/// Reject Unix pending notification cancellation until it is backed by a real host scheduler.
pub(super) fn cancel_pending_notification(
    _context: &RequestContext,
    _id: &str,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.pendingCancel"))
}

/// Reject Unix bulk pending notification cancellation until it is backed by a real host scheduler.
pub(super) fn cancel_all_pending_notifications() -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.pendingCancelAll"))
}

/// Validate Unix notification categories against the supported freedesktop action model.
pub(super) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    set_unix_categories(categories)
}

/// Remove active Unix notification state for one runtime id.
pub(super) fn unregister_runtime(host_session_id: HostSessionId) {
    unregister_notification_runtime(host_session_id);
}

/// Service freedesktop notification ingress.
pub(super) fn service_notification_ingress(_context: &SessionContext) -> RuntimeResult<()> {
    Ok(())
}

/// Apply runtime-registered action metadata to one outgoing Unix notification.
fn apply_notification_actions(
    notification: &mut Notification,
    host_session_id: HostSessionId,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    let Some(category_id) = request.category_id.as_deref() else {
        return Ok(());
    };
    let category = runtime::notification_category(host_session_id, category_id)?;

    let Some(category) = category else {
        return Ok(());
    };

    if !category.actions.is_empty() && !unix_notification_supports_actions()? {
        return Err(not_supported("destack.os.notification.categorySet"));
    }

    // attach every runtime category action as one freedesktop button
    for action in &category.actions {
        notification.action(&action.id, &action.title);
    }

    Ok(())
}
