use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::GlobalNodeIdAny;

/// Elaborated type guard checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuardTable {
    /// Runtime guard entry by guard node.
    pub entry_by_node: IndexMap<GlobalNodeIdAny, GuardEntry>,
}

impl GuardTable {
    /// Create an empty guard table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the guard entry for a node.
    pub fn set_entry(&mut self, node_id: GlobalNodeIdAny, entry: GuardEntry) {
        self.entry_by_node.insert(node_id, entry);
    }

    /// Get the guard entry for a node.
    pub fn entry(&self, node_id: GlobalNodeIdAny) -> Option<GuardEntry> {
        self.entry_by_node.get(&node_id).copied()
    }
}

/// Runtime check selected for one elaborated type guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardEntry {
    /// The runtime check was reduced to a constant.
    Constant(bool),
    /// The runtime check uses a union tag.
    UnionTag,
    /// The runtime check compares type identities.
    TypeDescriptor,
}
