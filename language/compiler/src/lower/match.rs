use dyst_ast::{self as ast};
use dyst_dir::{MatchCase, Module, NodeId, ScopeId, ScopeKind, SymbolSpace};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a match case to a DIR match case.
    pub(super) fn lower_match_case(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        match_case_id: ast::NodeId<ast::MatchCase>,
    ) -> NodeId<MatchCase> {
        let (symbol_id, scope_id) = self.session.tree.create_symbol_with_scope(
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
                let pattern = self.lower_pattern(module, scope_id, *pattern);
                let body = self.lower_expression(module, scope_id, *body);
                let guard = guard.map(|guard| self.lower_expression(module, scope_id, guard));
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
                let pattern = self.lower_pattern(module, scope_id, *pattern);
                let body = self.lower_block(module, scope_id, *body);
                let guard = guard.map(|guard| self.lower_expression(module, scope_id, guard));
                MatchCase::Block {
                    pattern,
                    body,
                    guard,
                    scope: scope_id,
                }
            }
        };
        self.session.tree.insert_from_source_as_symbol(
            match_case,
            module.id,
            match_case_id,
            symbol_id,
        )
    }
}
