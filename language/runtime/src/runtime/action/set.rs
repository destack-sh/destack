use rustc_hash::FxHashSet;

use crate::runtime::action::{Action, ActionId};

/// Set of canonical action identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionSet {
    /// Stored action identifiers.
    identifiers: FxHashSet<ActionId>,
}

impl ActionSet {
    /// Create one empty action set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create one action set from one iterator of action names.
    pub fn from_names<T>(names: impl IntoIterator<Item = T>) -> Self
    where
        T: AsRef<str>,
    {
        let mut set = Self::new();
        set.extend_names(names);

        set
    }

    /// Create one action set from one iterator of action identifiers.
    pub fn from_ids(ids: impl IntoIterator<Item = ActionId>) -> Self {
        ids.into_iter().collect()
    }

    /// Create one action set from one iterator of action kinds.
    pub fn from_actions(actions: impl IntoIterator<Item = Action>) -> Self {
        let action_ids = actions.into_iter().map(Action::id);
        Self::from_ids(action_ids)
    }

    /// Insert one action identifier.
    pub fn insert_id(&mut self, action: ActionId) -> bool {
        self.identifiers.insert(action)
    }

    /// Insert one action name.
    pub fn insert_name(&mut self, action_name: &str) -> bool {
        self.insert_id(ActionId::from_name(action_name))
    }

    /// Insert one action kind.
    pub fn insert_action(&mut self, action: Action) -> bool {
        self.insert_id(action.id())
    }

    /// Extend this set from one iterator of action names.
    pub fn extend_names<T>(&mut self, names: impl IntoIterator<Item = T>)
    where
        T: AsRef<str>,
    {
        for name in names {
            self.insert_name(name.as_ref());
        }
    }

    /// Extend this set from one iterator of action kinds.
    pub fn extend_actions(&mut self, actions: impl IntoIterator<Item = Action>) {
        for action in actions {
            self.insert_action(action);
        }
    }

    /// Return whether this set contains one action identifier.
    pub fn contains_id(&self, action: ActionId) -> bool {
        self.identifiers.contains(&action)
    }

    /// Return whether this set contains one action name.
    pub fn contains_name(&self, action_name: &str) -> bool {
        self.contains_id(ActionId::from_name(action_name))
    }

    /// Return whether this set contains one action kind.
    pub fn contains_action(&self, action: Action) -> bool {
        self.contains_id(action.id())
    }

    /// Return the number of actions in this set.
    pub fn len(&self) -> usize {
        self.identifiers.len()
    }

    /// Return whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.identifiers.is_empty()
    }

    /// Iterate the action identifiers in this set.
    pub fn iter(&self) -> impl Iterator<Item = &ActionId> {
        self.identifiers.iter()
    }
}

impl FromIterator<ActionId> for ActionSet {
    fn from_iter<T: IntoIterator<Item = ActionId>>(iter: T) -> Self {
        let mut set = Self::new();
        set.identifiers.extend(iter);

        set
    }
}

impl FromIterator<Action> for ActionSet {
    fn from_iter<T: IntoIterator<Item = Action>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend_actions(iter);
        set
    }
}

#[cfg(test)]
mod tests {
    use super::ActionSet;
    use crate::runtime::action::{Action, ActionId};

    #[test]
    fn test_from_actions_contains_action_kinds_and_ids() {
        let set = ActionSet::from_actions([Action::FsRead, Action::NetConnect]);

        assert!(set.contains_action(Action::FsRead));
        assert!(set.contains_action(Action::NetConnect));
        assert!(set.contains_id(ActionId::from_name("host.fs.read")));
        assert!(set.contains_id(ActionId::from_name("host.net.connect")));
    }

    #[test]
    fn test_extend_actions_merges_into_existing_set() {
        let mut set = ActionSet::from_names(["host.gpu.device"]);
        set.extend_actions([Action::GpuQueue, Action::GpuPresent]);

        assert!(set.contains_name("host.gpu.device"));
        assert!(set.contains_action(Action::GpuQueue));
        assert!(set.contains_action(Action::GpuPresent));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_insert_action_deduplicates_existing_action() {
        let mut set = ActionSet::new();
        let first_insert = set.insert_action(Action::IoPoll);
        let second_insert = set.insert_action(Action::IoPoll);

        assert!(first_insert);
        assert!(!second_insert);
        assert_eq!(set.len(), 1);
    }
}
