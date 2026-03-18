mod calendar;
mod contact;
mod document;
mod location;
mod permission;

use crate::diagnostic::RuntimeResult;
use crate::host::app::background::submit_background_request;
use crate::host::app::media::submit_media_request;
use crate::host::app::notification::submit_notification_request;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
#[cfg(target_os = "macos")]
pub(crate) use crate::host::macos::request::location::unregister_location_runtime;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use crate::host::macos::request::{
    calendar::{MacosCalendarHooks, set_macos_calendar_test_hooks},
    contact::{MacosContactHooks, set_macos_contact_test_hooks},
    document::set_macos_document_test_pick_hook,
    location::{MacosLocationHooks, set_macos_location_test_hooks},
};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
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
    if let Some(outcome) = location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    if let Some(outcome) = submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS calendar lanes
    if let Some(outcome) = calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS contact lanes
    if let Some(outcome) = contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    // then service macOS permission lanes
    if let Some(outcome) = permission::submit_permission_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise handle macOS-specific request lanes
    match request {
        HostRequest::OsDocumentPick { options } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentDescriptors(document::pick_documents(context, &options)?),
        )),
        _ => submit_unix_request(context, request),
    }
}
