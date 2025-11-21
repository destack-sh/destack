use dyst_dir::{ModuleId, NodeTree, Path, Scope, ScopeId, SymbolId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Path.
    pub(super) fn resolve_path(
        &self,
        module_id: ModuleId,
        scope: &Scope,
        path: &Path,
        tree: &NodeTree,
    ) -> ResolveResult<SymbolId> {
        todo!("resolve_path({path:?})")
    }
}
