use crate::host::os::linux::request;
use crate::runtime::action::ActionSet;

/// Return the static Linux host actions.
pub(crate) fn static_actions() -> ActionSet {
    ActionSet::new()
}

/// Return the runtime-dependent Linux host actions.
pub(crate) fn session_actions() -> ActionSet {
    request::request_actions()
}
