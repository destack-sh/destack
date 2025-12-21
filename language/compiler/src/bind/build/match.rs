use destack_ast::{self as ast};
use destack_dir::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, MatchCase, MatchSelector, NodeTree,
    NodeType, ScopeKind, SymbolBinding, SymbolTable, TypeTable,
};

use crate::Compiler;

use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind an AST MatchSelector to a DIR MatchSelector.
    fn bind_match_selector(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope_id: LocalScopeId,
        ast_selector: &ast::MatchSelector,
        parent_id: LocalNodeIdAny,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> MatchSelector {
        match ast_selector {
            ast::MatchSelector::Pattern {
                pattern: ast_pattern,
                guard: ast_guard,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    None,
                    SymbolBinding::Runtime,
                    *ast_pattern,
                    Some(parent_id),
                    tree,
                    symbols,
                    types,
                );
                let guard = ast_guard.map(|guard| {
                    self.bind_expression(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        guard,
                        Some(parent_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                MatchSelector::Pattern { pattern, guard }
            }
            ast::MatchSelector::Default => MatchSelector::Default,
        }
    }

    /// Bind a match case to a DIR match case.
    pub(super) fn bind_match_case(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_match_case_id: ast::LocalNodeId<ast::MatchCase>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<MatchCase> {
        let ast_match_case = ast.tree.get(ast_match_case_id);
        let match_case_id =
            tree.reserve_from_source(NodeType::MatchCase, ast_match_case_id.id, scope, parent_id);
        let (symbol_id, scope_id) =
            self.bind_anonymous_local_with_scope(module, ast, ScopeKind::Block, scope, symbols);
        let match_case = match ast_match_case {
            ast::MatchCase::Expression {
                selector: ast_selector,
                body,
            } => {
                let selector = self.bind_match_selector(
                    module,
                    ast,
                    scope_id,
                    ast_selector,
                    match_case_id,
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_expression(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(match_case_id),
                    tree,
                    symbols,
                    types,
                );
                MatchCase::Expression {
                    selector,
                    body,
                    scope: scope_id,
                }
            }
            ast::MatchCase::Block {
                selector: ast_selector,
                body,
            } => {
                let selector = self.bind_match_selector(
                    module,
                    ast,
                    scope_id,
                    ast_selector,
                    match_case_id,
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_block(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(match_case_id),
                    tree,
                    symbols,
                    types,
                );
                MatchCase::Block {
                    selector,
                    body,
                    scope: scope_id,
                }
            }
        };
        let match_case_id = tree.insert(match_case_id, match_case);
        symbols
            .get_symbol_mut(symbol_id)
            .declare_primary(match_case_id);
        match_case_id
    }
}
