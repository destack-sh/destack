use crate::diagnostic::RuntimeResult;
use crate::host::app::background::submit_background_request;
use crate::host::app::media::submit_media_request;
use crate::host::app::notification::submit_notification_request;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::platform::core::not_supported;
use crate::platform::os::{Permission, PermissionEntry, PermissionState};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic macOS request capabilities.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = unix_request_capabilities();
    capabilities.extend_capabilities([
        PlatformCapability::OsBackgroundControl,
        PlatformCapability::OsBackgroundRead,
        PlatformCapability::OsCalendarRead,
        PlatformCapability::OsCalendarWrite,
        PlatformCapability::OsContactRead,
        PlatformCapability::OsContactWrite,
        PlatformCapability::OsDocumentControl,
        PlatformCapability::OsDocumentPick,
        PlatformCapability::OsDocumentWrite,
        PlatformCapability::OsLocationRead,
        PlatformCapability::OsLocationWatch,
        PlatformCapability::OsMediaRead,
        PlatformCapability::OsMediaWrite,
        PlatformCapability::OsNotificationPermission,
        PlatformCapability::OsNotificationPost,
    ]);

    // document picking is live on the concrete macOS host
    capabilities.insert_capability(PlatformCapability::OsDocumentPick);

    capabilities
}

/// Submit one normalized macOS host request.
pub(crate) fn submit_request(
    context: &HostRequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    // service shared desktop background requests first
    if let Some(outcome) = submit_background_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop media requests
    if let Some(outcome) = submit_media_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS location lanes
    if let Some(outcome) = super::location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    if let Some(outcome) = submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS calendar lanes
    if let Some(outcome) = super::calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS contact lanes
    if let Some(outcome) = super::contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise handle macOS-specific request lanes
    match request {
        HostRequest::OsDocumentPick { options } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentDescriptors(super::document::pick_documents(
                context, &options,
            )?),
        )),
        HostRequest::OsPermissionRequest { permission } => {
            let Some(state) = request_permission_state(context, permission)? else {
                return Err(not_supported(request.operation_name()));
            };

            Ok(HostRequestOutcome::immediate(
                HostRequestResult::PermissionState(state),
            ))
        }
        HostRequest::OsPermissionRequestMany { ref permissions } => {
            let Some(entries) = request_permission_entries(context, permissions)? else {
                return Err(not_supported(request.operation_name()));
            };

            Ok(HostRequestOutcome::immediate(
                HostRequestResult::PermissionEntries(entries),
            ))
        }
        _ => submit_unix_request(context, request),
    }
}

/// Request one supported macOS permission selector.
fn request_permission_state(
    context: &HostRequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    if let Some(state) = super::location::request_location_permission(context, permission)? {
        return Ok(Some(state));
    }

    #[cfg(all(not(test), target_os = "macos"))]
    {
        if let Some(state) = super::calendar::permission::request_calendar_permission(permission)? {
            return Ok(Some(state));
        }

        if let Some(state) = super::contact::permission::request_contact_permission(permission)? {
            return Ok(Some(state));
        }
    }

    Ok(None)
}

/// Request one supported macOS permission selector batch.
fn request_permission_entries(
    context: &HostRequestContext,
    permissions: &[Permission],
) -> RuntimeResult<Option<Vec<PermissionEntry>>> {
    #[cfg(all(test, target_os = "macos"))]
    {
        let mut entries = Vec::with_capacity(permissions.len());

        // materialize one location result per requested selector
        for permission in permissions {
            let Some(state) = super::location::request_location_permission(context, *permission)?
            else {
                return Ok(None);
            };
            entries.push(PermissionEntry {
                permission: *permission,
                state,
            });
        }

        Ok(Some(entries))
    }

    #[cfg(all(not(test), target_os = "macos"))]
    {
        let mut entries = Vec::with_capacity(permissions.len());

        let mut calendar_permissions = Vec::new();
        let mut contact_permissions = Vec::new();
        let mut location_permissions = Vec::new();

        // partition selectors by family first
        for permission in permissions {
            match permission {
                crate::platform::os::Permission::CalendarRead | Permission::CalendarWrite => {
                    calendar_permissions.push(*permission);
                }
                Permission::ContactsRead | Permission::ContactsWrite => {
                    contact_permissions.push(*permission);
                }
                Permission::Location | Permission::LocationBackground => {
                    location_permissions.push(*permission);
                }
                _ => return Ok(None),
            }
        }

        if !calendar_permissions.is_empty() {
            let Some(calendar_entries) =
                super::calendar::permission::request_calendar_permissions(&calendar_permissions)?
            else {
                return Ok(None);
            };
            entries.extend(calendar_entries);
        }

        if !contact_permissions.is_empty() {
            let Some(contact_entries) =
                super::contact::permission::request_contact_permissions(&contact_permissions)?
            else {
                return Ok(None);
            };
            entries.extend(contact_entries);
        }

        if !location_permissions.is_empty() {
            let selected_permission = if location_permissions
                .iter()
                .any(|permission| matches!(permission, Permission::LocationBackground))
            {
                Permission::LocationBackground
            } else {
                Permission::Location
            };
            let Some(selected_state) =
                super::location::request_location_permission(context, selected_permission)?
            else {
                return Ok(None);
            };

            for permission in location_permissions {
                let state = match (permission, selected_state) {
                    (
                        crate::platform::os::Permission::Location,
                        crate::platform::os::PermissionState::Limited,
                    ) => crate::platform::os::PermissionState::Granted,
                    _ => selected_state,
                };
                entries.push(PermissionEntry { permission, state });
            }
        }

        Ok(Some(entries))
    }
}
