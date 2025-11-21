use dyst_dir::{ModuleId, Path, ScopeId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Path.
    pub(super) fn resolve_path(
        &self,
        _module_id: ModuleId,
        _scope_id: ScopeId,
        path: Path,
    ) -> ResolveResult<Path> {
        todo!("resolve_path({path:?})")
    }
}
