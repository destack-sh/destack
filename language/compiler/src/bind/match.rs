use dyst_ast::{self as ast};
use dyst_dir::{
    LocalNodeId, LocalScopeId, MatchCase, Module, NodeTree, ScopeKind, SymbolSpace, SymbolTable,
};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Bind a match case to a DIR match case.
    pub(super) fn bind_match_case(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        match_case_id: ast::LocalNodeId<ast::MatchCase>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<MatchCase> {
        let (symbol_id, scope_id) = symbols.insert_symbol_with_scope(
            SymbolSpace::Value,
            None,
            ScopeKind::Block,
            scope_id,
        );
        let match_case = module.get(match_case_id);
        let match_case = match match_case {
            ast::MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree, symbols);
                let body = self.bind_expression(module, scope_id, *body, tree, symbols);
                let guard =
                    guard.map(|guard| self.bind_expression(module, scope_id, guard, tree, symbols));
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
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree, symbols);
                let body = self.bind_block(module, scope_id, *body, tree, symbols);
                let guard =
                    guard.map(|guard| self.bind_expression(module, scope_id, guard, tree, symbols));
                MatchCase::Block {
                    pattern,
                    body,
                    guard,
                    scope: scope_id,
                }
            }
        };
        let match_case_id = tree.insert_from_source(match_case, match_case_id, scope_id);
        symbols.set_primary_declaration(symbol_id, match_case_id);
        match_case_id
    }
}
