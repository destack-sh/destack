use dyst_dir::{LocalScopeId, LocalSymbolId, ModuleId, NodeTree, Path, Scope};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Path.
    pub(super) fn resolve_path(
        &self,
        module_id: ModuleId,
        scope: &Scope,
        path: &Path,
        tree: &NodeTree,
    ) -> ResolveResult<LocalSymbolId> {
        todo!("resolve_path({path:?})")
    }
}
