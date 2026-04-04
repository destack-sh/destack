use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::submit_test_calendar_request;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return dynamic Unix calendar request capabilities for Linux tests.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = PlatformCapabilitySet::default();
    capabilities.insert_capability(PlatformCapability::OsCalendarRead);
    capabilities.insert_capability(PlatformCapability::OsCalendarWrite);

    capabilities
}

/// Submit one Unix calendar request through the Linux test lane.
pub(crate) fn submit_calendar_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_calendar_request(context, request)
}
