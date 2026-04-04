use crate::diagnostic::RuntimeResult;
use crate::host::os::windows::request::location::service::windows_location_service;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::os::{Permission, PermissionState};

/// Submit one Windows location request through the shared WinRT service.
pub(crate) fn submit_location_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let service = windows_location_service(request.operation_name())?;

    match request {
        // one services-enabled read
        HostRequest::OsLocationServicesEnabled => {
            let is_enabled = service.location_services_enabled(request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_enabled),
            )))
        }

        // one last-known read
        HostRequest::OsLocationLastKnown => {
            let sample = service
                .read_last_known_location(context.host_session_id, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }

        // one watch-open request
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            service.open_location_watch(
                context.host_session_id,
                watch_id.clone(),
                *options,
                request.operation_name(),
            )?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // one watch-close request
        HostRequest::OsLocationWatchClose { watch_id } => {
            service.close_location_watch(
                context.host_session_id,
                watch_id,
                request.operation_name(),
            )?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Request one supported Windows location permission selector.
pub(crate) fn request_location_permission(
    context: &RequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    if !matches!(permission, Permission::Location) {
        return Ok(None);
    }

    let service = windows_location_service("destack.os.permission.request")?;
    let state = service.request_location_permission(
        context.host_session_id,
        permission,
        "destack.os.permission.request",
    )?;

    Ok(Some(state))
}
