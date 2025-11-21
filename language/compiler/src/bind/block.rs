use dyst_ast::{self as ast};
use dyst_dir::{Block, Module, NodeId, NodeTree, ScopeId, ScopeKind, SymbolSpace};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Bind a block to a DIR block.
    pub(super) fn bind_block(
        &self,
        module: &Module,
        scope_id: ScopeId,
        block_id: ast::NodeId<ast::Block>,
        tree: &mut NodeTree,
    ) -> NodeId<Block> {
        let (symbol_id, scope_id) =
            tree.create_symbol_with_scope(SymbolSpace::Value, None, ScopeKind::Block, scope_id);
        let block = module.get(block_id);
        let label = block
            .label
            .map(|label| self.session.strings.intern_from(&module.ast_strings, label));
        let expressions = block
            .expressions
            .iter()
            .map(|expression| self.bind_expression(module, scope_id, *expression, tree))
            .collect();
        tree.insert_from_source_as_symbol(
            Block {
                label,
                expressions,
                scope: scope_id,
            },
            module.id,
            block_id,
            symbol_id,
        )
    }
}
