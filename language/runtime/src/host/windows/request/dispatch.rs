use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::core::not_supported;
use crate::platform::os::PermissionEntry;
use crate::runtime::action::ActionSet;

/// Return dynamic Windows request actions.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one normalized Windows host request.
pub(crate) fn submit_request(
    context: &RequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    let operation_name = request.operation_name();

    // service shared desktop background requests first
    if let Some(outcome) = super::background::submit_background_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop media requests
    if let Some(outcome) = super::media::submit_media_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop location requests
    if let Some(outcome) = super::location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    if let Some(outcome) = super::notification::submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Windows calendar requests
    if let Some(outcome) = super::calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Windows contact requests
    if let Some(outcome) = super::contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise handle Windows-specific requests
    match request {
        HostRequest::OsIntentCanOpenUrl { url } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::Bool(super::intent::windows_can_open_url(&url)?),
        )),
        HostRequest::OsIntentOpenUrl { url } => {
            super::intent::windows_open_url(&url)?;

            Ok(HostRequestOutcome::immediate(HostRequestResult::None))
        }
        HostRequest::OsIntentOpenPath { path } => {
            super::intent::windows_open_path_target(path)?;

            Ok(HostRequestOutcome::immediate(HostRequestResult::None))
        }
        HostRequest::OsDocumentPick { options } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentDescriptors(super::document::pick_documents(
                context, &options,
            )?),
        )),
        HostRequest::OsPermissionRequest { permission } => {
            let Some(state) = super::location::request_location_permission(context, permission)?
            else {
                return Err(not_supported(operation_name));
            };

            Ok(HostRequestOutcome::immediate(
                HostRequestResult::PermissionState(state),
            ))
        }
        HostRequest::OsPermissionRequestMany { permissions } => {
            let mut entries = Vec::with_capacity(permissions.len());

            // resolve one explicit permission result per requested selector
            for permission in permissions {
                let Some(state) =
                    super::location::request_location_permission(context, permission)?
                else {
                    return Err(not_supported(operation_name));
                };
                entries.push(PermissionEntry { permission, state });
            }

            Ok(HostRequestOutcome::immediate(
                HostRequestResult::PermissionEntries(entries),
            ))
        }
        _ => Err(not_supported(operation_name)),
    }
}
