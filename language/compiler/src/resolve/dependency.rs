use dyst_dir::{DependencyItem, ModuleId, NodeId};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a DependencyItem.
    pub fn resolve_dependency_item(
        &mut self,
        _module_id: ModuleId,
        item_id: NodeId<DependencyItem>,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: item_id.into(),
        })
    }
}
