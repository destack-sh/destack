use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::host::HostSessionId;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::HostActionSet;

/// Return dynamic Unix desktop request actions.
pub(crate) fn request_actions() -> HostActionSet {
    let mut actions = super::intent::request_actions();
    let location_actions = super::location::request_actions();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let calendar_actions = super::calendar::request_actions();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let contact_actions = super::contact::request_actions();

    #[cfg(all(unix, not(target_vendor = "apple")))]
    let document_actions = super::document::request_actions();

    // then merge any host-specific document actions
    #[cfg(all(unix, not(target_vendor = "apple")))]
    for action in document_actions.iter() {
        actions.insert_id(*action);
    }

    // then merge any host-specific location actions
    for action in location_actions.iter() {
        actions.insert_id(*action);
    }

    // then merge any host-specific calendar actions
    #[cfg(all(unix, not(target_vendor = "apple")))]
    for action in calendar_actions.iter() {
        actions.insert_id(*action);
    }

    // then merge any host-specific contact actions
    #[cfg(all(unix, not(target_vendor = "apple")))]
    for action in contact_actions.iter() {
        actions.insert_id(*action);
    }

    actions
}

/// Submit one normalized Unix desktop host request.
pub(crate) fn submit_request(
    context: &RequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    // service shared desktop background requests first
    #[cfg(target_os = "linux")]
    if let Some(outcome) = super::background::submit_background_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop media requests
    if let Some(outcome) = super::media::submit_media_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Unix calendar requests
    #[cfg(all(unix, not(target_vendor = "apple")))]
    if let Some(outcome) = super::calendar::submit_calendar_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Unix contact requests
    #[cfg(all(unix, not(target_vendor = "apple")))]
    if let Some(outcome) = super::contact::submit_contact_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Unix document requests
    #[cfg(all(unix, not(target_vendor = "apple")))]
    if let Some(outcome) = super::document::submit_document_request(context, &request)? {
        return Ok(outcome);
    }

    // then service Unix location requests
    if let Some(outcome) = super::location::submit_location_request(context, &request)? {
        return Ok(outcome);
    }

    // then service shared desktop notification requests
    #[cfg(all(unix, not(target_vendor = "apple")))]
    if let Some(outcome) = super::notification::submit_notification_request(context, &request)? {
        return Ok(outcome);
    }

    // otherwise fall through to Unix intent handling
    super::intent::submit_request(request)
}

/// Remove one Unix location runtime from the active backend.
#[cfg(target_os = "linux")]
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    super::location::unregister_location_runtime(host_session_id);
}
