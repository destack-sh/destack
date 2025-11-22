use crate::Compiler;
use dyst_ast::{self as ast, StringId};
use dyst_dir::{Module, Path};
use dyst_source::SmallVec;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Bind a path to a DIR path.
    pub(super) fn bind_path(&self, module: &Module, path: &ast::Path) -> Path {
        assert!(!path.segments.is_empty());
        let segments: SmallVec<StringId, 3> = path
            .segments
            .iter()
            .map(|segment| {
                self.session
                    .strings
                    .intern_from(&module.ast_strings, *segment)
            })
            .collect();
        Path { segments }
    }
}
