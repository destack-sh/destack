mod document;
mod intent;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
#[cfg(all(test, windows))]
pub(crate) use crate::host::windows::request::document::set_test_pick_hook;
use crate::platform::core::not_supported;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic Windows request capabilities.
pub(crate) fn session_capabilities() -> PlatformCapabilitySet {
    PlatformCapabilitySet::from_capabilities([
        PlatformCapability::OsDocumentPick,
        PlatformCapability::OsIntentWrite,
    ])
}

/// Submit one normalized Windows host request.
pub(crate) fn submit_request(request: HostRequest) -> RuntimeResult<HostRequestOutcome> {
    match request {
        HostRequest::OsIntentCanOpenUrl { url } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::Bool(intent::windows_can_open_url(&url)?),
        )),
        HostRequest::OsIntentOpenUrl { url } => {
            intent::windows_open_url(&url)?;

            Ok(HostRequestOutcome::immediate(HostRequestResult::None))
        }
        HostRequest::OsIntentOpenPath { path } => {
            intent::windows_open_path_target(path)?;

            Ok(HostRequestOutcome::immediate(HostRequestResult::None))
        }
        HostRequest::OsDocumentPick { options } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::DocumentDescriptors(document::pick_documents(&options)?),
        )),
        _ => Err(not_supported(request.operation_name())),
    }
}
