use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::linux::submit_test_contact_request;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic Unix contact capabilities for Linux tests.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    PlatformCapabilitySet::from_capabilities([
        PlatformCapability::OsContactRead,
        PlatformCapability::OsContactWrite,
    ])
}

/// Submit one Linux contact request through the active test lane.
pub(crate) fn submit_contact_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_contact_request(context, request)
}
