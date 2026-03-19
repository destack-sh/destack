use std::sync::OnceLock;

use notify_rust::{Hint, Notification, Timeout, Urgency};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::app::identity::{resolved_application_identifier, resolved_display_name};
use crate::host::app::notification::runtime;
use crate::host::core::error::not_supported;
use crate::host::core::{HostRequestContext, HostRuntimeId};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationRequestValue, NotificationScheduledDescriptorValue,
};
use crate::platform::os::{NotificationPermissionState, NotificationPriority};

use super::capability::{
    close_notification, unix_notification_server_available, unix_notification_supports_actions,
};
use super::category::set_categories as set_unix_categories;
use super::response::spawn_notification_response_worker;

static ACTIVE_UNIX_NOTIFICATIONS: OnceLock<Mutex<ActiveUnixNotificationRegistry>> = OnceLock::new();

/// Runtime-scoped Unix notification handles.
#[derive(Debug, Default)]
pub(super) struct ActiveUnixNotificationRegistry {
    /// Active notifications grouped by runtime id.
    pub(super) runtimes: FxHashMap<HostRuntimeId, FxHashMap<String, ActiveUnixNotification>>,
}

/// Active Unix notification state.
#[derive(Debug, Clone, Copy)]
pub(super) struct ActiveUnixNotification {
    /// Notification server identifier for CloseNotification.
    pub(super) server_id: u32,
}

/// Return the Unix desktop notification permission state.
pub(super) fn request_permission(
    _context: &HostRequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    if unix_notification_server_available() {
        return Ok(NotificationPermissionState::Granted);
    }

    Ok(NotificationPermissionState::Denied)
}

/// Deliver one notification through the Unix desktop host.
pub(super) fn deliver_notification(
    context: &HostRequestContext,
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
    apply_notification_actions(&mut notification, context.host_runtime_id, request)?;

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

    // track the server identifier before waiting for response signals
    {
        let registry = active_notification_registry();
        let mut registry = registry.lock();
        let runtime_notifications = registry
            .runtimes
            .entry(context.host_runtime_id)
            .or_default();

        runtime_notifications.insert(id.to_string(), ActiveUnixNotification { server_id });
    }

    // then wait for host action or close signals on a detached thread
    spawn_notification_response_worker(
        context.host_runtime_id,
        context.platform,
        id.to_string(),
        request.clone(),
        server_id,
    );

    Ok(())
}

/// Cancel one delivered Unix notification by its host identifier when present.
pub(super) fn cancel_notification(context: &HostRequestContext, id: &str) -> RuntimeResult<()> {
    let active_notification = {
        let registry = active_notification_registry();
        let mut registry = registry.lock();
        let mut remove_runtime = false;
        let active_notification = registry
            .runtimes
            .get_mut(&context.host_runtime_id)
            .and_then(|runtime_notifications| {
                let active_notification = runtime_notifications.remove(id);

                if runtime_notifications.is_empty() {
                    remove_runtime = true;
                }

                active_notification
            });

        if remove_runtime {
            registry.runtimes.remove(&context.host_runtime_id);
        }

        active_notification
    };

    let Some(active_notification) = active_notification else {
        return Ok(());
    };

    close_notification(active_notification.server_id)
}

/// Reject Unix notification scheduling until it is backed by a real host scheduler.
pub(super) fn schedule_notification(
    _context: &HostRequestContext,
    _id: &str,
    _request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.schedule"))
}

/// Reject Unix pending notification enumeration until it is backed by a real host scheduler.
pub(super) fn list_pending_notifications(
    _context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    Err(not_supported("destack.os.notification.pendingList"))
}

/// Reject Unix pending notification cancellation until it is backed by a real host scheduler.
pub(super) fn cancel_pending_notification(
    _context: &HostRequestContext,
    _id: &str,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.pendingCancel"))
}

/// Reject Unix bulk pending notification cancellation until it is backed by a real host scheduler.
pub(super) fn cancel_all_pending_notifications() -> RuntimeResult<()> {
    Err(not_supported("destack.os.notification.pendingCancelAll"))
}

/// Validate Unix notification categories against the supported freedesktop action model.
pub(super) fn set_categories(
    categories: &[crate::platform::os::abi_generated::NotificationCategoryValue],
) -> RuntimeResult<()> {
    set_unix_categories(categories)
}

/// Remove active Unix notification state for one runtime id.
pub(super) fn unregister_runtime(host_runtime_id: HostRuntimeId) {
    let registry = active_notification_registry();
    let mut registry = registry.lock();
    registry.runtimes.remove(&host_runtime_id);
}

/// Service freedesktop notification ingress.
pub(super) fn service_notification_ingress(_context: &HostRequestContext) -> RuntimeResult<()> {
    Ok(())
}

/// Return the shared active Unix notification registry.
pub(super) fn active_notification_registry() -> &'static Mutex<ActiveUnixNotificationRegistry> {
    ACTIVE_UNIX_NOTIFICATIONS.get_or_init(|| Mutex::new(ActiveUnixNotificationRegistry::default()))
}

/// Apply runtime-registered action metadata to one outgoing Unix notification.
fn apply_notification_actions(
    notification: &mut Notification,
    host_runtime_id: HostRuntimeId,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    let Some(category_id) = request.category_id.as_deref() else {
        return Ok(());
    };
    let category = runtime::notification_category(host_runtime_id, category_id)?;

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
