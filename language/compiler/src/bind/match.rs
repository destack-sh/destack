use dyst_ast::{self as ast};
use dyst_dir::{
    LocalNodeId, LocalScopeId, LocalScopeMark, MatchCase, Module, NodeTree, ScopeKind, SymbolTable,
    TypeTable,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a match case to a DIR match case.
    pub(super) fn bind_match_case(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        match_case_id: ast::LocalNodeId<ast::MatchCase>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<MatchCase> {
        let (symbol_id, scope_id) =
            self.bind_anonymous_local_with_scope(module, ScopeKind::Block, scope, symbols);
        let match_case = module.ast.get(match_case_id);
        let match_case = match match_case {
            ast::MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *pattern,
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_expression(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    tree,
                    symbols,
                    types,
                );
                let guard = guard.map(|guard| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        guard,
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
                    *pattern,
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_block(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    tree,
                    symbols,
                    types,
                );
                let guard = guard.map(|guard| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        guard,
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
        let match_case_id = tree.insert_from_source(match_case, match_case_id, scope);
        symbols
            .get_symbol_mut(symbol_id)
            .declare_primary(match_case_id);
        match_case_id
    }
}
