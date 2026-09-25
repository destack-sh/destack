use std::collections::{BTreeMap, BTreeSet};
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};

use crate::{LocalNodeIdAny, Tree};

/// A durable overlay over one base MIR tree.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Patch {
    /// The patch name.
    pub name: String,
    /// The rewritten MIR tree.
    pub tree: Tree,
    /// Replacement roots keyed by the base node they replace.
    pub replacement_by_node: BTreeMap<LocalNodeIdAny, LocalNodeIdAny>,
    /// Node ids hidden by this patch.
    pub dead_nodes: BTreeSet<LocalNodeIdAny>,
}

impl Patch {
    /// Create a named patch from an already rewritten tree.
    pub fn from_tree(name: impl Into<String>, tree: Tree) -> Self {
        Self {
            name: name.into(),
            tree,
            replacement_by_node: BTreeMap::new(),
            dead_nodes: BTreeSet::new(),
        }
    }

    /// Return the replacement node for one base node.
    #[inline]
    pub fn replacement_for(&self, node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        self.replacement_by_node.get(&node_id).copied()
    }

    /// Replace one visible root with another visible root.
    #[inline]
    pub fn replace(&mut self, source: LocalNodeIdAny, target: LocalNodeIdAny) {
        self.replacement_by_node.insert(source, target);
    }

    /// Delete one visible node from this patch.
    #[inline]
    pub fn delete(&mut self, node_id: LocalNodeIdAny) {
        self.dead_nodes.insert(node_id);
    }

    /// Return whether one node is hidden by this patch.
    #[inline]
    pub fn is_dead(&self, node_id: LocalNodeIdAny) -> bool {
        self.dead_nodes.contains(&node_id)
    }
}
