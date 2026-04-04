#[cfg(target_os = "ios")]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "ios")]
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};

/// Submit one outbound iOS host request.
#[cfg(target_os = "ios")]
pub(crate) fn submit_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let runtime_id = context.host_session_id.0;

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

    if let Some(outcome) = super::document::submit::submit_document_request(context, request)? {
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

    if let Some(outcome) = super::permission::submit::submit_permission_request(context, request)? {
        return Ok(Some(outcome));
    }

    super::notification::submit::submit_notification_request(runtime_id, request)
}
