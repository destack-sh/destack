use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::Path;
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a path to a DIR path.
    pub fn lower_path(
        &mut self,
        source_id: SourceId,
        _ast: &ast::NodeTree,
        path: &ast::Path,
    ) -> Path {
        Path::String {
            segments: path
                .segments
                .iter()
                .map(|segment| self.intern_string(source_id, *segment))
                .collect(),
        }
    }
}
