use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::core::not_supported;
use crate::platform::os::{Permission, PermissionEntry, PermissionState};
use crate::runtime::action::{HostAction, HostActionSet};

/// Return dynamic macOS request actions.
pub(crate) fn request_actions() -> HostActionSet {
    let mut actions = request::request_actions();
    actions.extend_actions([
        HostAction::OsBackgroundControl,
        HostAction::OsBackgroundRead,
        HostAction::OsCalendarRead,
        HostAction::OsCalendarWrite,
        HostAction::OsContactRead,
        HostAction::OsContactWrite,
        HostAction::OsDocumentControl,
        HostAction::OsDocumentPick,
        HostAction::OsDocumentWrite,
        HostAction::OsLocationRead,
        HostAction::OsLocationWatch,
        HostAction::OsMediaRead,
        HostAction::OsMediaWrite,
        HostAction::OsNotificationPermission,
        HostAction::OsNotificationPost,
    ]);

    // document picking is live on the concrete macOS host
    actions.insert_action(HostAction::OsDocumentPick);

    actions
}

/// Submit one normalized macOS host request.
pub(crate) fn submit_request(
    context: &RequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    // service shared desktop background requests first
    if let Some(outcome) = super::background::submit_background_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop media requests
    if let Some(outcome) = super::media::submit_media_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS location requests
    if let Some(outcome) = super::location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    if let Some(outcome) = super::notification::submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS calendar requests
    if let Some(outcome) = super::calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS contact requests
    if let Some(outcome) = super::contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise handle macOS-specific requests
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
        _ => request::submit_request(context, request),
    }
}

/// Request one supported macOS permission selector.
fn request_permission_state(
    context: &RequestContext,
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
    context: &RequestContext,
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
                Permission::CalendarRead | Permission::CalendarWrite => {
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
                    (Permission::Location, PermissionState::Limited) => PermissionState::Granted,
                    _ => selected_state,
                };
                entries.push(PermissionEntry { permission, state });
            }
        }

        Ok(Some(entries))
    }
}
