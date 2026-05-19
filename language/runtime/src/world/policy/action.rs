use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use destack_core::fnv1a_128;

/// Stable identifier for one canonical action name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(
    /// Stable hash of the canonical action string.
    pub u128,
);

impl ActionId {
    /// Build one action identifier from one action name.
    pub const fn from_name(name: &str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}

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

    /// Create one action set from action names.
    pub fn from_names<T>(names: impl IntoIterator<Item = T>) -> Self
    where
        T: AsRef<str>,
    {
        let mut set = Self::new();
        set.extend_names(names);

        set
    }

    /// Insert one action name.
    pub fn insert_name(&mut self, action_name: &str) -> bool {
        self.insert_id(ActionId::from_name(action_name))
    }

    /// Extend this set from action names.
    pub fn extend_names<T>(&mut self, names: impl IntoIterator<Item = T>)
    where
        T: AsRef<str>,
    {
        for name in names {
            self.insert_name(name.as_ref());
        }
    }

    /// Return whether this set contains one action name.
    pub fn contains_name(&self, action_name: &str) -> bool {
        self.contains_id(ActionId::from_name(action_name))
    }

    /// Return whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.identifiers.is_empty()
    }

    /// Insert one action identifier.
    fn insert_id(&mut self, action: ActionId) -> bool {
        self.identifiers.insert(action)
    }

    /// Return whether this set contains one action identifier.
    fn contains_id(&self, action: ActionId) -> bool {
        self.identifiers.contains(&action)
    }
}
