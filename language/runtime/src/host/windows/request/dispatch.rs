use crate::diagnostic::RuntimeResult;
use crate::host::app::background::submit_background_request;
use crate::host::app::document::access::{
    list_document_access_grants, persist_document_access_grants, revoke_document_access_grants,
};
use crate::host::app::document::storage::import_document_descriptors_to_app_storage;
use crate::host::app::media::submit_media_request;
use crate::host::app::notification::submit_notification_request;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::platform::core::not_supported;
use crate::platform::os::PermissionEntry;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic Windows request capabilities.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = PlatformCapabilitySet::from_capabilities([
        PlatformCapability::OsBackgroundControl,
        PlatformCapability::OsBackgroundRead,
        PlatformCapability::OsCalendarRead,
        PlatformCapability::OsCalendarWrite,
        PlatformCapability::OsContactRead,
        PlatformCapability::OsContactWrite,
        PlatformCapability::OsDocumentControl,
        PlatformCapability::OsDocumentPick,
        PlatformCapability::OsDocumentWrite,
        PlatformCapability::OsIntentWrite,
        PlatformCapability::OsLocationRead,
        PlatformCapability::OsLocationWatch,
        PlatformCapability::OsMediaRead,
        PlatformCapability::OsMediaWrite,
    ]);
    capabilities.extend_capabilities([
        PlatformCapability::OsNotificationPermission,
        PlatformCapability::OsNotificationPost,
    ]);

    capabilities
}

/// Submit one normalized Windows host request.
pub(crate) fn submit_request(
    context: &HostRequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    let operation_name = request.operation_name();

    // service shared desktop background requests first
    if let Some(outcome) = submit_background_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop media requests
    if let Some(outcome) = submit_media_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop location requests
    if let Some(outcome) = super::location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    if let Some(outcome) = submit_notification_request(context, &request)? {
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

    // otherwise handle Windows-specific request lanes
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
        HostRequest::OsDocumentImport { documents } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentDescriptors(import_document_descriptors_to_app_storage(
                context, documents,
            )?),
        )),
        HostRequest::OsDocumentAccessPersist { documents, access } => Ok(
            HostRequestOutcome::immediate(HostRequestResult::DocumentAccessGrants(
                persist_document_access_grants(context, documents, access)?,
            )),
        ),
        HostRequest::OsDocumentAccessList => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentAccessGrants(list_document_access_grants(context)?),
        )),
        HostRequest::OsDocumentAccessRevoke { ids } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::U32(revoke_document_access_grants(context, &ids)?),
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
