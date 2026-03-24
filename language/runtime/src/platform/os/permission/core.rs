use crate::diagnostic::RuntimeResult;
use crate::platform::os::state::notification_request_permission;
use crate::platform::os::{
    NotificationPermissionState, Permission, PermissionEntry, PermissionState,
};
use crate::runtime::BindingCallContext;

use super::state as permission_state;

/// Open host settings for runtime permissions.
pub(crate) fn open_settings(binding: &BindingCallContext) -> RuntimeResult<()> {
    permission_state::permission_open_settings(binding)
}

/// Request host authorization for one permission selector.
pub(crate) fn request(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    permission_state::permission_request(binding, permission)
}

/// Request host authorization for one permission selector list.
pub(crate) fn request_many(
    binding: &BindingCallContext,
    permissions: Vec<Permission>,
) -> RuntimeResult<Vec<PermissionEntry>> {
    permission_state::permission_request_many(binding, permissions)
}

/// Read one runtime-owned permission state.
pub(crate) fn state(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    permission_state::permission_state(binding, permission)
}

/// Read multiple runtime-owned permission states.
pub(crate) fn state_many(
    binding: &BindingCallContext,
    permissions: &[Permission],
) -> RuntimeResult<Vec<PermissionEntry>> {
    permission_state::permission_state_many(binding, permissions)
}

/// Read the runtime-owned notification permission state.
pub(crate) fn notification_state(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    permission_state::notification_permission_state(binding)
}

/// Request host notification permission.
pub(crate) fn request_notification(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    notification_request_permission(binding)
}
