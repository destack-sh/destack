use dyst_ast::{self as ast};
use dyst_dir::{Block, Module, NodeId, ScopeId, ScopeKind, SymbolSpace};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a block to a DIR block.
    pub(super) fn lower_block(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        block_id: ast::NodeId<ast::Block>,
    ) -> NodeId<Block> {
        let (symbol_id, scope_id) = self.session.tree.create_symbol_with_scope(
            SymbolSpace::Value,
            None,
            ScopeKind::Block,
            scope_id,
        );
        let block = module.get(block_id);
        let label = block
            .label
            .map(|label| self.session.strings.intern_from(&module.strings, label));
        let expressions = block
            .expressions
            .iter()
            .map(|expression| self.lower_expression(module, scope_id, *expression))
            .collect();
        self.session.tree.insert_from_source_as_symbol(
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
