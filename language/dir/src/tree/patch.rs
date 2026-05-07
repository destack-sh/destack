use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{LocalNodeIdAny, LocalSymbolId, Tree};

/// A durable overlay over one base DIR tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// The owning module id.
    pub module_id: ModuleId,
    /// The tree containing nodes introduced by this patch.
    pub tree: Tree,
    /// Replacement roots keyed by the base node they replace.
    pub replace: Vec<(LocalNodeIdAny, LocalNodeIdAny)>,
    /// Node ids hidden by this patch.
    pub dead: Vec<LocalNodeIdAny>,
    /// Symbol ids hidden by this patch.
    pub inactive_symbols: Vec<LocalSymbolId>,
    /// Parent overrides keyed by the visible child node.
    pub parent: Vec<(LocalNodeIdAny, Option<LocalNodeIdAny>)>,
}

impl Patch {
    /// Create an empty patch over one base tree.
    pub fn new(base: &Tree) -> Self {
        Self {
            module_id: base.module_id,
            tree: Tree::with_first_global_id(base.module_id, base.next_global_id(), 0),
            replace: Vec::new(),
            dead: Vec::new(),
            inactive_symbols: Vec::new(),
            parent: Vec::new(),
        }
    }

    /// Return whether this patch applies to one base tree.
    #[inline]
    pub fn applies_to(&self, base: &Tree) -> bool {
        self.module_id == base.module_id && self.tree.first_global_id() == base.next_global_id()
    }

    /// Return the replacement node for one base node.
    #[inline]
    pub fn replacement_for(&self, node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        self.replace
            .iter()
            .find_map(|(source, target)| (*source == node_id).then_some(*target))
    }

    /// Replace one visible root with another visible root.
    #[inline]
    pub fn replace(&mut self, source: LocalNodeIdAny, target: LocalNodeIdAny) {
        if let Some((_, existing_target)) = self.replace.iter_mut().find(|(id, _)| *id == source) {
            *existing_target = target;
        } else {
            self.replace.push((source, target));
        }
    }

    /// Delete one visible node from this patch.
    #[inline]
    pub fn delete(&mut self, node_id: LocalNodeIdAny) {
        if !self.dead.contains(&node_id) {
            self.dead.push(node_id);
        }
    }

    /// Return whether one node is hidden by this patch.
    #[inline]
    pub fn is_dead(&self, node_id: LocalNodeIdAny) -> bool {
        self.dead.contains(&node_id)
    }

    /// Hide one symbol from this patch.
    #[inline]
    pub fn deactivate_symbol(&mut self, symbol_id: LocalSymbolId) {
        if !self.inactive_symbols.contains(&symbol_id) {
            self.inactive_symbols.push(symbol_id);
        }
    }

    /// Return whether one symbol is hidden by this patch.
    #[inline]
    pub fn symbol_is_inactive(&self, symbol_id: LocalSymbolId) -> bool {
        self.inactive_symbols.contains(&symbol_id)
    }

    /// Return the parent override for one node.
    #[inline]
    pub fn parent_for(&self, node_id: LocalNodeIdAny) -> Option<Option<LocalNodeIdAny>> {
        self.parent
            .iter()
            .find_map(|(child, parent)| (*child == node_id).then_some(*parent))
    }

    /// Override the visible parent for one node.
    #[inline]
    pub fn set_parent(&mut self, node_id: LocalNodeIdAny, parent_id: Option<LocalNodeIdAny>) {
        if let Some((_, existing_parent)) = self.parent.iter_mut().find(|(id, _)| *id == node_id) {
            *existing_parent = parent_id;
        } else {
            self.parent.push((node_id, parent_id));
        }
    }

    /// Return whether this patch owns one node id.
    #[inline]
    pub fn has_node(&self, node_id: LocalNodeIdAny) -> bool {
        self.tree.has_node_id(node_id.id)
    }
}
