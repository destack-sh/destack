use serde::{Deserialize, Serialize};
use tspp_core::{FxIndexMap as IndexMap, FxIndexSet as IndexSet};
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{LocalNodeIdAny, Tree, View};

/// A durable overlay over one base DIR tree.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Patch {
    /// The patch name.
    pub name: String,
    /// The owning module id.
    pub module_id: ModuleId,
    /// The tree containing nodes introduced by this patch.
    pub tree: Tree,
    /// Replacement roots keyed by the base node they replace.
    pub replacement_by_node: IndexMap<LocalNodeIdAny, LocalNodeIdAny>,
    /// Node ids deleted by this patch.
    pub deleted_nodes: IndexSet<LocalNodeIdAny>,
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
            replacement_by_node: IndexMap::default(),
            deleted_nodes: IndexSet::default(),
            parent_by_node: IndexMap::default(),
        }
    }

    /// Create an empty patch whose node ids follow every node one view shows.
    pub fn following(view: &View<'_>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            module_id: view.tree().module_id,
            tree: Tree::following(view.tree().module_id, view.next_global_id()),
            replacement_by_node: IndexMap::default(),
            deleted_nodes: IndexSet::default(),
            parent_by_node: IndexMap::default(),
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
        assert_eq!(
            source.ty, target.ty,
            "DIR patch cannot replace {source:?} with a different node type {target:?}"
        );

        self.replacement_by_node.insert(source, target);
    }

    /// Delete one visible node from this patch.
    #[inline]
    pub fn delete(&mut self, node_id: LocalNodeIdAny) {
        self.deleted_nodes.insert(node_id);
    }

    /// Return whether one node is deleted by this patch.
    #[inline]
    pub fn is_deleted(&self, node_id: LocalNodeIdAny) -> bool {
        self.deleted_nodes.contains(&node_id)
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
