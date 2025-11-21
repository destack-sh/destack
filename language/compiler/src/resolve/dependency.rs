use dyst_dir::{DependencyItem, LocalNodeId, ModuleId, NodeTree};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a DependencyItem.
    pub(super) fn resolve_dependency_item(
        &self,
        _module_id: ModuleId,
        item_id: LocalNodeId<DependencyItem>,
        _tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: item_id.into(),
        })
    }
}
