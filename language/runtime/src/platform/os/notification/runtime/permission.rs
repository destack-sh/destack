use crate::diagnostic::RuntimeResult;
use crate::host::RequestContext;
use crate::platform::os::NotificationPermissionState;

use super::state::{DesktopNotificationRuntimeState, notification_runtime_service};
use crate::platform::os::notification::delivery;

/// Return the default notification permission state for one platform.
pub(crate) fn request_notification_permission(
    context: &RequestContext,
) -> RuntimeResult<NotificationPermissionState> {
    let permission_state = delivery::request_notification_permission(context)?;

    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(context.host_session_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
    runtime_state.permission_state = permission_state;

    Ok(permission_state)
}
