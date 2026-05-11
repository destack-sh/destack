use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::submit_test_calendar_request;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::{HostAction, HostActionSet};

/// Return dynamic Unix calendar request actions for Linux tests.
pub(crate) fn request_actions() -> HostActionSet {
    let mut actions = HostActionSet::default();
    actions.insert_action(HostAction::OsCalendarRead);
    actions.insert_action(HostAction::OsCalendarWrite);

    actions
}

/// Submit one Unix calendar request through the Linux test lane.
pub(crate) fn submit_calendar_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_calendar_request(context, request)
}
