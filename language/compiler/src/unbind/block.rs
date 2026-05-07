use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR block to an AST block.
    pub(super) fn unbind_block(
        &self,
        module: &Module,
        block_id: dir::LocalNodeId<dir::Block>,
        block_context: ast::BlockContext,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Block> {
        let block = tree.get(block_id);
        let span = self.unbind_span(module, block_id.into());
        let leading_expressions = block
            .leading_expressions
            .iter()
            .map(|expression| {
                self.unbind_expression(
                    module,
                    *expression,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                )
            })
            .collect();
        let tail_expression = block.tail_expression.map(|expression| {
            self.unbind_expression(
                module,
                expression,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            )
        });
        let ast_block = ast::Block {
            context: block_context,
            form: ast::BlockForm::Explicit,
            leading_expressions,
            tail_expression,
        };
        let ast_block_id = ast_tree.insert(ast_block, span);
        context.map(block_id.into_any(), ast_block_id.into_any());
        ast_block_id
    }
}
