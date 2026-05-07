use destack_artifact::Ast;
use destack_ast::{self as ast};
use destack_dir::{
    Block, BlockContext, BlockForm, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark,
    ModuleBinding, NodeType, ScopeKind, SymbolSpace, SymbolTable, Tree, TypeTable,
};
use destack_workspace::Module;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a block to a DIR block.
    pub(super) fn bind_block(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_block_id: ast::LocalNodeId<ast::Block>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Block> {
        let ast_block = ast.tree.get(ast_block_id);
        let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
            module,
            ast,
            ScopeKind::Block,
            scope,
            None,
            symbols,
        );
        let block_id = tree.reserve_from_source(
            NodeType::Block,
            ast_block_id.id,
            (scope_id, LocalScopeMark::end()),
            parent_id,
        );
        let leading_expressions = ast_block
            .leading_expressions
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *expression,
                    Some(block_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                )
            })
            .collect();
        let tail_expression = ast_block.tail_expression.map(|expression| {
            self.bind_expression(
                module,
                ast,
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
                (scope_id, symbols.get_scope_mark(scope_id)),
                expression,
                Some(block_id),
                tree,
                symbols,
                types,
                SymbolSpace::Value,
            )
        });
        let block_id = tree.insert(
            block_id,
            Block {
                context: BlockContext::Expression,
                form: BlockForm::Explicit,
                leading_expressions,
                tail_expression,
                scope: scope_id,
            },
        );
        symbols.get_symbol_mut(symbol_id).declare(block_id);
        block_id
    }
}
