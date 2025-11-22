use dyst_dir::{DependencyItem, LocalNodeId, Module, NodeTree, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a DependencyItem.
    pub(super) fn resolve_dependency_item(
        &self,
        module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: item_id.into_global_any(module.id),
        })
    }
}
