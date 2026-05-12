use crate::host::os::windows::request;
use crate::runtime::action::{Action, ActionSet};

/// Return the static Windows host actions.
pub(crate) fn static_actions() -> ActionSet {
    let mut host_actions = ActionSet::new();

    // windows exposes runtime host intent ingress callbacks
    host_actions.insert_action(Action::OsPowerRead);

    host_actions
}

/// Return the runtime-dependent Windows host actions.
pub(crate) fn session_actions() -> ActionSet {
    request::request_actions()
}
