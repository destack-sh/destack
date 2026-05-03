use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR path to an AST path.
    pub(super) fn unbind_path(
        &self,
        path: &dir::Path,
        _ast_strings: &mut StringPool,
        _context: &mut UnbindContext,
    ) -> ast::Path {
        let segments = path.segments.iter().map(|segment| *segment).collect();
        ast::Path { segments }
    }
}
