use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::{LocalNodeIdAny, Tree};

/// A durable overlay over one base DIR tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// The patch name.
    pub name: String,
    /// The owning module id.
    pub module_id: ModuleId,
    /// The tree containing nodes introduced by this patch.
    pub tree: Tree,
    /// Replacement roots keyed by the base node they replace.
    pub replacement_by_node: IndexMap<LocalNodeIdAny, LocalNodeIdAny>,
    /// Node ids hidden by this patch.
    pub dead_nodes: IndexSet<LocalNodeIdAny>,
    /// Parent overrides keyed by the visible child node.
    pub parent_by_node: IndexMap<LocalNodeIdAny, Option<LocalNodeIdAny>>,
}

impl Patch {
    /// Create an empty patch over one base tree.
    pub fn new(base: &Tree, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            module_id: base.module_id,
            tree: Tree::from_base(base, 0),
            replacement_by_node: IndexMap::new(),
            dead_nodes: IndexSet::new(),
            parent_by_node: IndexMap::new(),
        }
    }

    /// Return the replacement node for one base node.
    #[inline]
    pub fn replacement_for(&self, node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        self.replacement_by_node.get(&node_id).copied()
    }

    /// Iterate replacement root nodes.
    #[inline]
    pub fn replacement_targets(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.replacement_by_node.values().copied()
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

    /// Return the parent override for one node.
    #[inline]
    pub fn parent_for(&self, node_id: LocalNodeIdAny) -> Option<Option<LocalNodeIdAny>> {
        self.parent_by_node.get(&node_id).copied()
    }

    /// Override the visible parent for one node.
    #[inline]
    pub fn set_parent(&mut self, node_id: LocalNodeIdAny, parent_id: Option<LocalNodeIdAny>) {
        self.parent_by_node.insert(node_id, parent_id);
    }

    /// Return whether this patch owns one node id.
    #[inline]
    pub fn has_node(&self, node_id: LocalNodeIdAny) -> bool {
        self.tree.has_node_id(node_id.id)
    }
}
