use dyst_dir::{DependencyItem, LocalNodeId, ModuleId, NodeTree, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a DependencyItem.
    pub(super) fn resolve_dependency_item(
        &self,
        module_id: ModuleId,
        item_id: LocalNodeId<DependencyItem>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: item_id.into_global_any(module_id),
        })
    }
}
