use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::windows::tests::submit_contact_request as submit_contact_request_from_tests;
pub(crate) use crate::host::windows::tests::{WindowsContactHooks, set_windows_contact_test_hooks};

/// Submit one Windows contact request through the active test lane.
pub(crate) fn submit_contact_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_contact_request_from_tests(context, request)
}
