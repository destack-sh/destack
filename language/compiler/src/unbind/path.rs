use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR path to an AST path.
    pub(super) fn unbind_path(&self, path: &dir::Path, ast_strings: &mut StringPool) -> ast::Path {
        let segments = path
            .segments
            .iter()
            .map(|segment| ast_strings.intern_from(&self.program.strings, *segment))
            .collect();
        ast::Path { segments }
    }
}
