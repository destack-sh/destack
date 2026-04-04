use std::sync::mpsc::channel;

use block2::RcBlock;
use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::os::apple::call::call_process_main_context_if_needed;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{Permission, PermissionEntry, PermissionState};

/// Request one supported macOS calendar permission.
pub(in crate::host::os::macos::request) fn request_calendar_permission(
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    let Some(permission_kind) = calendar_permission_kind(permission) else {
        return Ok(None);
    };
    let authorization_status = request_calendar_authorization_status(permission_kind)?;
    let permission_state = permission_state_from_calendar_status(permission, authorization_status);

    Ok(Some(permission_state))
}

/// Request one supported macOS calendar permission batch.
pub(in crate::host::os::macos::request) fn request_calendar_permissions(
    permissions: &[Permission],
) -> RuntimeResult<Option<Vec<PermissionEntry>>> {
    if permissions.iter().any(|permission| {
        !matches!(
            permission,
            Permission::CalendarRead | Permission::CalendarWrite
        )
    }) {
        return Ok(None);
    }

    let authorization_status = if permissions.is_empty() {
        EKAuthorizationStatus::NotDetermined
    } else {
        let permission_kind = if permissions
            .iter()
            .any(|permission| matches!(permission, Permission::CalendarRead))
        {
            CalendarPermissionKind::Read
        } else {
            CalendarPermissionKind::Write
        };

        request_calendar_authorization_status(permission_kind)?
    };

    let entries = permissions
        .iter()
        .map(|permission| PermissionEntry {
            permission: *permission,
            state: permission_state_from_calendar_status(*permission, authorization_status),
        })
        .collect();

    Ok(Some(entries))
}

/// Request one calendar authorization status through EventKit.
fn request_calendar_authorization_status(
    permission_kind: CalendarPermissionKind,
) -> RuntimeResult<EKAuthorizationStatus> {
    call_process_main_context_if_needed(move || {
        let store = unsafe { EKEventStore::new() };
        let (sender, receiver) = channel();
        let completion = RcBlock::new(
            move |_granted: objc2::runtime::Bool, error: *mut objc2_foundation::NSError| {
                let result = if error.is_null() {
                    let status = unsafe {
                        EKEventStore::authorizationStatusForEntityType(EKEntityType::Event)
                    };

                    Ok(status)
                } else {
                    Err(permission_error("EventKit permission request failed"))
                };

                if sender.send(result).is_err() {
                    tracing::warn!(
                        target: "destack.runtime.host.macos.permission",
                        "calendar permission callback result receiver dropped",
                    );
                }
            },
        );

        // use the narrowest request that satisfies the permission selector
        unsafe {
            match permission_kind {
                CalendarPermissionKind::Read => {
                    store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&completion));
                }
                CalendarPermissionKind::Write => {
                    store
                        .requestWriteOnlyAccessToEventsWithCompletion(RcBlock::as_ptr(&completion));
                }
            }
        }

        receiver.recv().map_err(|error| {
            permission_error(format!(
                "could not receive one EventKit permission result: {error}"
            ))
        })?
    })
}

/// Return the calendar authorization request shape for one runtime permission.
fn calendar_permission_kind(permission: Permission) -> Option<CalendarPermissionKind> {
    match permission {
        Permission::CalendarRead => Some(CalendarPermissionKind::Read),
        Permission::CalendarWrite => Some(CalendarPermissionKind::Write),
        _ => None,
    }
}

/// Map one EventKit authorization status into one runtime permission state.
fn permission_state_from_calendar_status(
    permission: Permission,
    status: EKAuthorizationStatus,
) -> PermissionState {
    match status {
        EKAuthorizationStatus::NotDetermined => PermissionState::Prompt,
        EKAuthorizationStatus::Restricted => PermissionState::Restricted,
        EKAuthorizationStatus::Denied => PermissionState::Denied,
        EKAuthorizationStatus::FullAccess => PermissionState::Granted,

        // write only access is still a useful partial success for read lanes
        EKAuthorizationStatus::WriteOnly => match permission {
            Permission::CalendarRead => PermissionState::Limited,
            Permission::CalendarWrite => PermissionState::Granted,
            _ => PermissionState::Denied,
        },
        _ => PermissionState::Denied,
    }
}

/// Build one loud EventKit permission error.
fn permission_error(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.permission.request: {}", message.into()),
    ))
    .boxed()
}

/// Calendar authorization request mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalendarPermissionKind {
    /// Full read access.
    Read,
    /// Write-only access.
    Write,
}
