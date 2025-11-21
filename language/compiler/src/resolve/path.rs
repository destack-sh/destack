use dyst_dir::{LocalSymbolId, ModuleId, NodeTree, Path, Scope};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Path.
    pub(super) fn resolve_path(
        &self,
        _module_id: ModuleId,
        _scope: &Scope,
        path: &Path,
        _tree: &NodeTree,
    ) -> ResolveResult<LocalSymbolId> {
        todo!("resolve_path({path:?})")
    }
}
