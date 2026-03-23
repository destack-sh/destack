use super::{HostOperation, decode};

use crate::host::core::HostRequest;
use crate::platform::os::{Permission, PermissionEntry, PermissionState};

/// Build one open-settings operation.
pub(crate) fn open_settings() -> HostOperation<()> {
    HostOperation::new(HostRequest::OsPermissionOpenSettings, decode::none)
}

/// Build one single-permission request operation.
pub(crate) fn request(permission: Permission) -> HostOperation<PermissionState> {
    HostOperation::new(
        HostRequest::OsPermissionRequest { permission },
        decode::permission_state,
    )
}

/// Build one multi-permission request operation.
pub(crate) fn request_many(permissions: Vec<Permission>) -> HostOperation<Vec<PermissionEntry>> {
    HostOperation::new(
        HostRequest::OsPermissionRequestMany { permissions },
        decode::permission_entries,
    )
}
