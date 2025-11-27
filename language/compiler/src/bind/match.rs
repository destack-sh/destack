use dyst_ast::{self as ast};
use dyst_dir::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, MatchCase, Module, NodeTree,
    NodeType, ScopeKind, SymbolTable, TypeTable,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a match case to a DIR match case.
    pub(super) fn bind_match_case(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        ast_match_case_id: ast::LocalNodeId<ast::MatchCase>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<MatchCase> {
        let ast_match_case = module.ast.get(ast_match_case_id);
        let match_case_id =
            tree.reserve_from_source(NodeType::MatchCase, ast_match_case_id, scope, parent_id);
        let (symbol_id, scope_id) =
            self.bind_anonymous_local_with_scope(module, ScopeKind::Block, scope, symbols);
        let match_case = match ast_match_case {
            ast::MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    None,
                    *pattern,
                    Some(match_case_id),
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_expression(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(match_case_id),
                    tree,
                    symbols,
                    types,
                );
                let guard = guard.map(|guard| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        guard,
                        Some(match_case_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                MatchCase::Expression {
                    pattern,
                    body,
                    guard,
                    scope: scope_id,
                }
            }
            ast::MatchCase::Block {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    None,
                    *pattern,
                    Some(match_case_id),
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_block(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(match_case_id),
                    tree,
                    symbols,
                    types,
                );
                let guard = guard.map(|guard| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        guard,
                        Some(match_case_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                MatchCase::Block {
                    pattern,
                    body,
                    guard,
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
