use destack_ast::{self as ast};
use destack_dir::{
    Block, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree, NodeType,
    ScopeKind, SymbolTable, TypeTable,
};
use destack_workspace::Module;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a block to a DIR block.
    pub(super) fn bind_block(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        ast_block_id: ast::LocalNodeId<ast::Block>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Block> {
        let ast_block = module.ast.tree.get(ast_block_id);
        let (symbol_id, scope_id) =
            self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, None, symbols);
        let block_id = tree.reserve_from_source(
            NodeType::Block,
            ast_block_id.id,
            (scope_id, LocalScopeMark::end()),
            parent_id,
        );
        let label = ast_block
            .label
            .map(|label| self.program.strings.intern_from(&module.ast.strings, label));
        let expressions = ast_block
            .expressions
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *expression,
                    Some(block_id),
                    tree,
                    symbols,
                    types,
                )
            })
            .collect();
        let block_id = tree.insert(
            block_id,
            Block {
                label,
                expressions,
                scope: scope_id,
            },
        );
        symbols.get_symbol_mut(symbol_id).declare_primary(block_id);
        block_id
    }
}
