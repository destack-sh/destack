pub(crate) mod intent;

#[cfg(target_os = "ios")]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "ios")]
use crate::host::core::{HostRequest, HostRequestOutcome};

pub use intent::*;

/// Submit one outbound iOS host request.
#[cfg(target_os = "ios")]
pub(crate) fn submit_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    intent::submit::submit_intent_request(runtime_id, request)
}
