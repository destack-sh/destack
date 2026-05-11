use crate::host::os::windows::request;
use crate::runtime::action::{HostAction, HostActionSet};

/// Return the static Windows host actions.
pub(crate) fn static_actions() -> HostActionSet {
    let mut host_actions = HostActionSet::new();

    // windows exposes runtime host intent ingress callbacks
    host_actions.insert_action(HostAction::OsLifecycleRead);
    host_actions.insert_action(HostAction::OsIntentRead);
    host_actions.insert_action(HostAction::OsPower);
    host_actions.insert_action(HostAction::OsPermissionRead);

    host_actions
}

/// Return the runtime-dependent Windows host actions.
pub(crate) fn session_actions() -> HostActionSet {
    request::request_actions()
}
