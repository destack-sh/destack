use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, Span};
use destack_workspace::Module;

use crate::Compiler;

/// Result of unbinding a module.
#[derive(Debug)]
pub struct UnboundModule {
    /// The AST tree.
    pub tree: ast::NodeTree,
    /// The string pool.
    pub strings: StringPool,
    /// The root expressions.
    pub roots: Vec<ast::LocalNodeId<ast::Expression>>,
}

impl Compiler {
    /// Get the span for a DIR node.
    #[inline]
    pub(super) fn unbind_span(&self, _module: &Module, _node_id: dir::LocalNodeIdAny) -> Span {
        // #Incomplete: resolve back to original AST span via source_node_id mapping?
        Span::empty(FileId::new(0))
    }

    /// Unbind a module's DIR tree to an AST tree.
    pub fn unbind_module(&self, module: &Module) -> UnboundModule {
        let dir = module.dir();
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // rebuild the AST tree
        let mut ast_tree = ast::NodeTree::new();
        let mut ast_strings = StringPool::new();
        let roots: Vec<ast::LocalNodeId<ast::Expression>> = dir
            .roots
            .iter()
            .map(|expression_id| {
                self.unbind_expression(
                    module,
                    *expression_id,
                    &tree,
                    &symbols,
                    &mut ast_tree,
                    &mut ast_strings,
                )
            })
            .collect();

        UnboundModule {
            tree: ast_tree,
            strings: ast_strings,
            roots,
        }
    }
}
