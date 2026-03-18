#[cfg(all(unix, not(target_vendor = "apple")))]
mod calendar;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod contact;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod document;
mod intent;
mod location;

use crate::diagnostic::RuntimeResult;
use crate::host::app::background::submit_background_request;
use crate::host::app::media::submit_media_request;
use crate::host::app::notification::submit_notification_request;
#[cfg(target_os = "linux")]
use crate::host::core::HostRuntimeId;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::runtime::capability::PlatformCapabilitySet;

/// Return dynamic Unix desktop request capabilities.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = intent::request_capabilities();
    capabilities.extend_capabilities(location::desktop_capabilities());
    let location_capabilities = location::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let calendar_capabilities = calendar::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let contact_capabilities = contact::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let document_capabilities = document::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then merge any host-specific document capabilities
    for capability in document_capabilities.iter() {
        capabilities.insert_id(*capability);
    }

    // then merge any host-specific location capabilities
    for capability in location_capabilities.iter() {
        capabilities.insert_id(*capability);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then merge any host-specific calendar capabilities
    for capability in calendar_capabilities.iter() {
        capabilities.insert_id(*capability);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then merge any host-specific contact capabilities
    for capability in contact_capabilities.iter() {
        capabilities.insert_id(*capability);
    }

    capabilities
}

/// Submit one normalized Unix desktop host request.
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

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service Unix calendar requests
    if let Some(outcome) = calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service Unix contact requests
    if let Some(outcome) = contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service Unix document requests
    if let Some(outcome) = document::submit_document_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Unix location requests
    if let Some(outcome) = location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    if let Some(outcome) = submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise fall through to the Unix intent lane
    intent::submit_request(request)
}

/// Remove one Unix location runtime from the active backend.
#[cfg(target_os = "linux")]
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    location::unregister_location_runtime(host_runtime_id);
}
