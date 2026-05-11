use crate::host::os::linux::request;
use crate::runtime::action::{HostAction, HostActionSet};

/// Return the static Linux host actions.
pub(crate) fn static_actions() -> HostActionSet {
    HostActionSet::new()
}

/// Return the runtime-dependent Linux host actions.
pub(crate) fn session_actions() -> HostActionSet {
    let mut actions = request::request_actions();
    actions.extend_actions([
        HostAction::OsBackgroundControl,
        HostAction::OsBackgroundRead,
    ]);

    actions
}
