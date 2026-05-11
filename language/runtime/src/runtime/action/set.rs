use rustc_hash::FxHashSet;

use crate::runtime::action::{HostAction, HostActionId};

/// Set of canonical host action identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostActionSet {
    /// Stored action identifiers.
    identifiers: FxHashSet<HostActionId>,
}

impl HostActionSet {
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
    pub fn from_ids(ids: impl IntoIterator<Item = HostActionId>) -> Self {
        ids.into_iter().collect()
    }

    /// Create one action set from one iterator of action kinds.
    pub fn from_actions(actions: impl IntoIterator<Item = HostAction>) -> Self {
        let action_ids = actions.into_iter().map(HostAction::id);
        Self::from_ids(action_ids)
    }

    /// Insert one action identifier.
    pub fn insert_id(&mut self, action: HostActionId) -> bool {
        self.identifiers.insert(action)
    }

    /// Insert one action name.
    pub fn insert_name(&mut self, action_name: &str) -> bool {
        self.insert_id(HostActionId::from_name(action_name))
    }

    /// Insert one action kind.
    pub fn insert_action(&mut self, action: HostAction) -> bool {
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
    pub fn extend_actions(&mut self, actions: impl IntoIterator<Item = HostAction>) {
        for action in actions {
            self.insert_action(action);
        }
    }

    /// Return whether this set contains one action identifier.
    pub fn contains_id(&self, action: HostActionId) -> bool {
        self.identifiers.contains(&action)
    }

    /// Return whether this set contains one action name.
    pub fn contains_name(&self, action_name: &str) -> bool {
        self.contains_id(HostActionId::from_name(action_name))
    }

    /// Return whether this set contains one action kind.
    pub fn contains_action(&self, action: HostAction) -> bool {
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
    pub fn iter(&self) -> impl Iterator<Item = &HostActionId> {
        self.identifiers.iter()
    }
}

impl FromIterator<HostActionId> for HostActionSet {
    fn from_iter<T: IntoIterator<Item = HostActionId>>(iter: T) -> Self {
        let mut set = Self::new();
        set.identifiers.extend(iter);

        set
    }
}

impl FromIterator<HostAction> for HostActionSet {
    fn from_iter<T: IntoIterator<Item = HostAction>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend_actions(iter);
        set
    }
}

#[cfg(test)]
mod tests {
    use super::HostActionSet;
    use crate::runtime::action::{HostAction, HostActionId};

    #[test]
    fn test_from_actions_contains_action_kinds_and_ids() {
        let set = HostActionSet::from_actions([HostAction::FsRead, HostAction::NetConnect]);

        assert!(set.contains_action(HostAction::FsRead));
        assert!(set.contains_action(HostAction::NetConnect));
        assert!(set.contains_id(HostActionId::from_name("fs.read")));
        assert!(set.contains_id(HostActionId::from_name("net.connect")));
    }

    #[test]
    fn test_extend_actions_merges_into_existing_set() {
        let mut set = HostActionSet::from_names(["gpu.device"]);
        set.extend_actions([HostAction::GpuQueue, HostAction::GpuPresent]);

        assert!(set.contains_name("gpu.device"));
        assert!(set.contains_action(HostAction::GpuQueue));
        assert!(set.contains_action(HostAction::GpuPresent));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_insert_action_deduplicates_existing_action() {
        let mut set = HostActionSet::new();
        let first_insert = set.insert_action(HostAction::IoPoll);
        let second_insert = set.insert_action(HostAction::IoPoll);

        assert!(first_insert);
        assert!(!second_insert);
        assert_eq!(set.len(), 1);
    }
}
