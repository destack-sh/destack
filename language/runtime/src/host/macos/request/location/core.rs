use crate::diagnostic::RuntimeResult;
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult, HostRuntimeId,
};
use crate::platform::os::{Permission, PermissionState};

use super::permission::{LocationPermissionRequest, request_location_permission_on_main};
use super::service::{
    close_location_watch, location_services_enabled, open_location_watch, read_last_known_location,
    unregister_location_runtime_on_main,
};

/// The location last-known operation name.
pub(super) const LOCATION_LAST_KNOWN_OPERATION: &str = "destack.os.location.lastKnown";

/// The location watch-open operation name.
pub(super) const LOCATION_WATCH_OPEN_OPERATION: &str = "destack.os.location.watchOpen";

/// The location watch-close operation name.
pub(super) const LOCATION_WATCH_CLOSE_OPERATION: &str = "destack.os.location.watchClose";

/// The location permission request operation name.
pub(super) const LOCATION_PERMISSION_REQUEST_OPERATION: &str = "destack.os.permission.request";

/// Submit one macOS location request through Core Location.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // read global service availability
        HostRequest::OsLocationServicesEnabled => {
            let is_enabled = location_services_enabled()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_enabled),
            )))
        }

        // read the most recent sample
        HostRequest::OsLocationLastKnown => {
            let sample = read_last_known_location(context.host_runtime_id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }

        // open one live watch
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            open_location_watch(context.host_runtime_id, watch_id, options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // close one live watch
        HostRequest::OsLocationWatchClose { watch_id } => {
            close_location_watch(context.host_runtime_id, watch_id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Request one supported macOS location permission selector.
pub(crate) fn request_location_permission(
    context: &HostRequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    let request_kind = match permission {
        Permission::Location => LocationPermissionRequest::WhenInUse,
        Permission::LocationBackground => LocationPermissionRequest::Always,
        _ => return Ok(None),
    };
    let permission_state =
        request_location_permission_on_main(context.host_runtime_id, permission, request_kind)?;

    Ok(Some(permission_state))
}

/// Remove one macOS runtime from the active location backend.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    unregister_location_runtime_on_main(host_runtime_id);
}
