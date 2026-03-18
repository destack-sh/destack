pub(crate) mod background;
pub(crate) mod calendar;
pub(crate) mod contact;
pub(crate) mod intent;
pub(crate) mod location;
pub(crate) mod media;
pub(crate) mod notification;

#[cfg(target_os = "android")]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "android")]
use crate::host::core::{HostRequest, HostRequestOutcome};

pub use background::*;
pub use calendar::*;
pub use contact::*;
pub use intent::*;
pub use location::*;
pub use media::*;
pub use notification::*;

/// Submit one outbound Android host request.
#[cfg(target_os = "android")]
pub(crate) fn submit_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    if let Some(outcome) = background::submit::submit_background_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = calendar::submit::submit_calendar_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = contact::submit::submit_contact_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = intent::submit::submit_intent_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = location::submit::submit_location_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    if let Some(outcome) = media::submit::submit_media_request(runtime_id, request)? {
        return Ok(Some(outcome));
    }

    notification::submit::submit_notification_request(runtime_id, request)
}
