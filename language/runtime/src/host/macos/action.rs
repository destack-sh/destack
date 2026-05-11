use crate::runtime::action::{HostAction, HostActionSet};

/// Return the static macOS host actions.
pub(crate) fn static_actions() -> HostActionSet {
    let mut host_actions = HostActionSet::new();

    host_actions.insert_action(HostAction::OsLifecycleRead);
    host_actions.insert_action(HostAction::OsIntentRead);
    host_actions.insert_action(HostAction::OsPower);
    host_actions.insert_action(HostAction::OsPermissionRead);

    host_actions
}
