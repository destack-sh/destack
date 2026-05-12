use crate::runtime::action::{Action, ActionSet};

/// Return the static macOS host actions.
pub(crate) fn static_actions() -> ActionSet {
    let mut host_actions = ActionSet::new();

    host_actions.insert_action(Action::OsPowerRead);

    host_actions
}
