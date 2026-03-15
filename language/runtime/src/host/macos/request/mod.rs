mod document;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
#[cfg(all(test, target_os = "macos"))]
pub(crate) use crate::host::macos::request::document::set_test_pick_hook;
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic macOS request capabilities.
pub(crate) fn session_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = unix_request_capabilities();

    // document picking is live on the concrete macOS host
    capabilities.insert_capability(PlatformCapability::OsDocumentPick);

    capabilities
}

/// Submit one normalized macOS host request.
pub(crate) fn submit_request(request: HostRequest) -> RuntimeResult<HostRequestOutcome> {
    match request {
        HostRequest::OsDocumentPick { options } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentDescriptors(document::pick_documents(&options)?),
        )),
        _ => submit_unix_request(request),
    }
}
