use dyst_dir::{ModuleId, Path, ScopeId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Path.
    pub fn resolve_path(
        &mut self,
        _module_id: ModuleId,
        _scope_id: ScopeId,
        path: Path,
    ) -> ResolveResult<Path> {
        todo!("resolve_path({path:?})")
    }
}
