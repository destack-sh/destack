use crate::diagnostic::RuntimeResult;
use crate::host::os::windows::tests::submit_calendar_request as submit_calendar_request_from_tests;
pub(crate) use crate::host::os::windows::tests::{
    WindowsCalendarHooks, set_windows_calendar_test_hooks,
};
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};

/// Submit one Windows calendar request through the active test lane.
pub(crate) fn submit_calendar_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_calendar_request_from_tests(context, request)
}
