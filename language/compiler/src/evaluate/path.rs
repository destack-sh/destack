use dyst_dir::{NodeIdAny, Path};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Analyze a Path.
    pub fn analyze_path(&mut self, _scope_id: NodeIdAny, path: Path) -> EvaluateResult<Path> {
        todo!("analyze_path({path:?})")
    }
}
