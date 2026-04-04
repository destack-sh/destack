use crate::diagnostic::RuntimeResult;
use crate::host::os::macos::tests::submit_calendar_request as submit_calendar_request_from_tests;
pub(crate) use crate::host::os::macos::tests::{MacosCalendarHooks, set_macos_calendar_test_hooks};
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};

/// Submit one macOS calendar request through the active test lane.
pub(crate) fn submit_calendar_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_calendar_request_from_tests(context, request)
}
