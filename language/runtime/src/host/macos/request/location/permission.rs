use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2_core_location::{CLAuthorizationStatus, CLLocationManager};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::apple::execution::with_process_main_context_marker_if_needed;
use crate::host::core::HostRuntimeId;
use crate::host::macos::macos_notify_permission_result;
use crate::platform::PlatformError;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{Permission, PermissionState};

use super::core::LOCATION_PERMISSION_REQUEST_OPERATION;
use super::delegate::MacosLocationManagerDelegate;

/// The permission wait slice for Core Location authorization changes.
const LOCATION_AUTHORIZATION_WAIT_SLICE: Duration = Duration::from_millis(25);

/// The permission wait deadline for Core Location authorization changes.
const LOCATION_AUTHORIZATION_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Core Location authorization request level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocationPermissionRequest {
    /// Request when-in-use authorization.
    WhenInUse,
    /// Request always authorization.
    Always,
}

/// Ensure Core Location authorization is present for one live request.
pub(super) fn ensure_location_authorized(
    manager: &CLLocationManager,
    host_runtime_id: HostRuntimeId,
    request_kind: LocationPermissionRequest,
    mtm: MainThreadMarker,
    operation: &'static str,
) -> RuntimeResult<CLAuthorizationStatus> {
    let authorization_status = unsafe { manager.authorizationStatus() };
    if is_location_authorized(authorization_status) {
        return Ok(authorization_status);
    }

    if !matches!(authorization_status, CLAuthorizationStatus::NotDetermined) {
        return Err(location_permission_denied(
            operation,
            location_permission_denied_message(authorization_status),
        ));
    }

    let delegate =
        MacosLocationManagerDelegate::new(mtm, host_runtime_id, None, false, authorization_status);

    unsafe {
        manager.setDelegate(Some(delegate.as_protocol()));
    }

    request_location_authorization(manager, request_kind);

    let authorization_status =
        wait_for_location_authorization(manager, delegate.as_ref(), operation)?;

    unsafe {
        manager.setDelegate(None);
    }

    if !is_location_authorized(authorization_status) {
        return Err(location_permission_denied(
            operation,
            location_permission_denied_message(authorization_status),
        ));
    }

    Ok(authorization_status)
}

/// Request one macOS location permission on the process main thread.
pub(super) fn request_location_permission_on_main(
    host_runtime_id: HostRuntimeId,
    permission: Permission,
    request_kind: LocationPermissionRequest,
) -> RuntimeResult<PermissionState> {
    with_process_main_context_marker_if_needed(move |mtm| {
        let manager = unsafe { CLLocationManager::new() };
        let initial_status = unsafe { manager.authorizationStatus() };
        let delegate =
            MacosLocationManagerDelegate::new(mtm, host_runtime_id, None, false, initial_status);

        unsafe {
            manager.setDelegate(Some(delegate.as_protocol()));
        }

        if matches!(initial_status, CLAuthorizationStatus::NotDetermined) {
            request_location_authorization(&manager, request_kind);
        }

        let authorization_status = wait_for_location_authorization(
            &manager,
            delegate.as_ref(),
            LOCATION_PERMISSION_REQUEST_OPERATION,
        )?;
        let permission_state =
            permission_state_from_location_status(permission, authorization_status);

        unsafe {
            manager.setDelegate(None);
        }

        macos_notify_permission_result(
            host_runtime_id.0,
            location_permission_name_for_permission(permission),
            matches!(permission_state, PermissionState::Granted),
        )?;

        Ok(permission_state)
    })
}

/// Request one Core Location authorization prompt at the selected level.
fn request_location_authorization(
    manager: &CLLocationManager,
    request_kind: LocationPermissionRequest,
) {
    unsafe {
        match request_kind {
            LocationPermissionRequest::WhenInUse => manager.requestWhenInUseAuthorization(),
            LocationPermissionRequest::Always => manager.requestAlwaysAuthorization(),
        }
    }
}

/// Wait for Core Location authorization to leave the not-determined state.
fn wait_for_location_authorization(
    manager: &CLLocationManager,
    delegate: &MacosLocationManagerDelegate,
    operation: &'static str,
) -> RuntimeResult<CLAuthorizationStatus> {
    let started_at = Instant::now();
    let run_loop = NSRunLoop::currentRunLoop();

    loop {
        let authorization_status = delegate
            .authorization_status()
            .unwrap_or_else(|| unsafe { manager.authorizationStatus() });

        if !matches!(authorization_status, CLAuthorizationStatus::NotDetermined) {
            return Ok(authorization_status);
        }

        if started_at.elapsed() >= LOCATION_AUTHORIZATION_WAIT_TIMEOUT {
            return Err(io_operation_error(
                operation,
                Some(PlatformErrorCode::IoWouldBlock),
                "timed out waiting for CoreLocation authorization",
            ));
        }

        // pump one small main-run-loop slice while the native prompt is active
        let limit_date =
            NSDate::dateWithTimeIntervalSinceNow(LOCATION_AUTHORIZATION_WAIT_SLICE.as_secs_f64());
        let default_mode = unsafe { NSDefaultRunLoopMode };

        run_loop.runMode_beforeDate(default_mode, limit_date.as_ref());
    }
}

/// Return whether one Core Location authorization status grants access.
pub(super) fn is_location_authorized(status: CLAuthorizationStatus) -> bool {
    matches!(
        status,
        CLAuthorizationStatus::AuthorizedAlways | CLAuthorizationStatus::AuthorizedWhenInUse
    )
}

/// Map one Core Location authorization status into one runtime permission state.
fn permission_state_from_location_status(
    permission: Permission,
    status: CLAuthorizationStatus,
) -> PermissionState {
    match status {
        CLAuthorizationStatus::NotDetermined => PermissionState::Prompt,
        CLAuthorizationStatus::Restricted => PermissionState::Restricted,
        CLAuthorizationStatus::Denied => PermissionState::Denied,
        CLAuthorizationStatus::AuthorizedAlways => PermissionState::Granted,

        // when-in-use is a partial success for background access
        CLAuthorizationStatus::AuthorizedWhenInUse => match permission {
            Permission::Location => PermissionState::Granted,
            Permission::LocationBackground => PermissionState::Limited,
            _ => PermissionState::Denied,
        },
        _ => PermissionState::Denied,
    }
}

/// Return the runtime permission name that matches one location authorization state.
pub(super) fn location_permission_name_for_status(status: CLAuthorizationStatus) -> &'static str {
    match status {
        CLAuthorizationStatus::AuthorizedAlways => "locationBackground",
        _ => "location",
    }
}

/// Return the runtime permission name for one requested selector.
fn location_permission_name_for_permission(permission: Permission) -> &'static str {
    match permission {
        Permission::LocationBackground => "locationBackground",
        _ => "location",
    }
}

/// Build one location permission-denied message for one native status.
fn location_permission_denied_message(status: CLAuthorizationStatus) -> &'static str {
    match status {
        CLAuthorizationStatus::Restricted => "CoreLocation access is restricted for this runtime",
        CLAuthorizationStatus::Denied => "CoreLocation access was denied for this runtime",
        CLAuthorizationStatus::NotDetermined => {
            "CoreLocation permission has not been granted for this runtime"
        }
        _ => "CoreLocation access is not available for this runtime",
    }
}

/// Build one loud location permission error.
fn location_permission_denied(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoPermissionDenied),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}
