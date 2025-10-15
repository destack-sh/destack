use dyst_dir::{NodeIdAny, Path};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Evaluate a Path.
    pub fn evaluate_path(&mut self, scope_id: NodeIdAny, path: Path) -> Path {
        todo!("evaluate_path({path:?})")
    }
}
