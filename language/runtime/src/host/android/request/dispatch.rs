#[cfg(target_os = "android")]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "android")]
use crate::host::core::{HostRequest, HostRequestOutcome};

/// Submit one outbound Android host request.
#[cfg(target_os = "android")]
pub(crate) fn submit_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    if let Some(outcome) =
        super::background::submit::submit_background_request(runtime_id, request)?
    {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = super::calendar::submit::submit_calendar_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = super::contact::submit::submit_contact_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = super::intent::submit::submit_intent_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = super::location::submit::submit_location_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = super::media::submit::submit_media_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    super::notification::submit::submit_notification_request(runtime_id, request)
}
