use dyst_dir::{NodeIdAny, Path};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Analyze a Path.
    pub fn analyze_path(&mut self, _scope_id: NodeIdAny, path: Path) -> ResolveResult<Path> {
        todo!("analyze_path({path:?})")
    }
}
