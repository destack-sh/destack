use dyst_ast::{self as ast};
use dyst_dir::{
    Block, LocalNodeId, LocalScopeId, Module, NodeTree, ScopeKind, SymbolSpace, SymbolTable,
    TypeTable,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a block to a DIR block.
    pub(super) fn bind_block(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        block_id: ast::LocalNodeId<ast::Block>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Block> {
        let (symbol_id, scope_id) =
            symbols.insert_symbol_with_scope(SymbolSpace::Value, None, ScopeKind::Block, scope_id);
        let block = module.get(block_id);
        let label = block
            .label
            .map(|label| self.program.strings.intern_from(&module.ast_strings, label));
        let expressions = block
            .expressions
            .iter()
            .map(|expression| {
                self.bind_expression(module, scope_id, *expression, tree, symbols, types)
            })
            .collect();
        let block_id = tree.insert_from_source(
            Block {
                label,
                expressions,
                scope: scope_id,
            },
            block_id,
            scope_id,
        );
        symbols.set_primary_declaration(symbol_id, block_id);
        block_id
    }
}
