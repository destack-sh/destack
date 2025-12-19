use crate::Compiler;
use destack_ast::{self as ast, StringId};
use destack_dir::Path;
use smallvec::SmallVec;

use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a path to a DIR path.
    pub(super) fn bind_path(&self, _module: &Module, ast: &ModuleAst, path: &ast::Path) -> Path {
        assert!(!path.segments.is_empty());
        let segments: SmallVec<[StringId; 3]> = path
            .segments
            .iter()
            .map(|segment| self.program.strings.intern_from(&ast.strings, *segment))
            .collect();
        Path { segments }
    }
}
