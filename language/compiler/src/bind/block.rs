use dyst_ast::{self as ast};
use dyst_dir::{
    Block, LocalNodeId, LocalScopeId, LocalScopeMark, Module, NodeTree, ScopeKind, SymbolTable,
    TypeTable,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a block to a DIR block.
    pub(super) fn bind_block(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        block_id: ast::LocalNodeId<ast::Block>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Block> {
        let (symbol_id, scope_id) =
            self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, None, symbols);
        let block = module.ast.get(block_id);
        let label = block
            .label
            .map(|label| self.program.strings.intern_from(&module.ast_strings, label));
        let expressions = block
            .expressions
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *expression,
                    tree,
                    symbols,
                    types,
                )
            })
            .collect();
        let block_id = tree.insert_from_source(
            Block {
                label,
                expressions,
                scope: scope_id,
            },
            block_id,
            (scope_id, LocalScopeMark::end()),
        );
        symbols.get_symbol_mut(symbol_id).declare_primary(block_id);
        block_id
    }
}
