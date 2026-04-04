use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::host::HostSessionId;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::capability::PlatformCapabilitySet;

/// Return dynamic Unix desktop request capabilities.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = super::intent::request_capabilities();
    let location_capabilities = super::location::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let calendar_capabilities = super::calendar::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let contact_capabilities = super::contact::request_capabilities();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let document_capabilities = super::document::request_capabilities();

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
    context: &RequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    #[cfg(target_os = "linux")]
    // service shared desktop background requests first
    if let Some(outcome) = super::background::submit_background_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop media requests
    if let Some(outcome) = super::media::submit_media_request(context, &request)? {
        return Ok(outcome);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service Unix calendar requests
    if let Some(outcome) = super::calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service Unix contact requests
    if let Some(outcome) = super::contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service Unix document requests
    if let Some(outcome) = super::document::submit_document_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Unix location requests
    if let Some(outcome) = super::location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    #[cfg(all(unix, not(target_vendor = "apple")))]
    // then service shared desktop notification requests
    if let Some(outcome) = super::notification::submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise fall through to the Unix intent lane
    super::intent::submit_request(request)
}

/// Remove one Unix location runtime from the active backend.
#[cfg(target_os = "linux")]
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    super::location::unregister_location_runtime(host_session_id);
}
