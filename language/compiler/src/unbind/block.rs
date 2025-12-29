use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR block to an AST block.
    pub(super) fn unbind_block(
        &self,
        module: &Module,
        block_id: dir::LocalNodeId<dir::Block>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Block> {
        let block = tree.get(block_id);
        let span = self.unbind_span(module, block_id.into());
        let expressions = block
            .expressions
            .iter()
            .map(|expression| {
                self.unbind_expression(
                    module,
                    *expression,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                )
            })
            .collect();
        let ast_block = ast::Block {
            format: ast::BlockFormat::Explicit,
            expressions,
        };
        let ast_block_id = ast_tree.insert(ast_block, span);
        context.map(block_id.into_any(), ast_block_id.into_any());
        ast_block_id
    }
}
