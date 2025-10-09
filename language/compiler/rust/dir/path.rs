use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{NodeId, Path};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a path to a DIR path.
    pub fn lower_path(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        path: &ast::Path,
    ) -> Path {
        // let path = document = ...
        todo!("Compiler::lower_path")
    }
}
