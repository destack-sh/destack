use crate::diagnostic::RuntimeResult;
use crate::host::os::macos::tests::submit_contact_request as submit_contact_request_from_tests;
pub(crate) use crate::host::os::macos::tests::{MacosContactHooks, set_macos_contact_test_hooks};
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};

/// Submit one macOS contact request through the active test lane.
pub(crate) fn submit_contact_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_contact_request_from_tests(context, request)
}
