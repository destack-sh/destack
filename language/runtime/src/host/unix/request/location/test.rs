use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRuntimeId};
use crate::host::linux::{submit_test_location_request, unregister_test_location_runtime};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic Unix location capabilities for Linux tests.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    PlatformCapabilitySet::from_capabilities([
        PlatformCapability::OsLocationRead,
        PlatformCapability::OsLocationWatch,
    ])
}

/// Submit one Linux location request through the active test lane.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_location_request(context, request)
}

/// Remove one Linux runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    unregister_test_location_runtime(host_runtime_id);
}
