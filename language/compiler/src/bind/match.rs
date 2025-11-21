use dyst_ast::{self as ast};
use dyst_dir::{MatchCase, Module, NodeId, NodeTree, ScopeId, ScopeKind, SymbolSpace};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Bind a match case to a DIR match case.
    pub(super) fn bind_match_case(
        &self,
        module: &Module,
        scope_id: ScopeId,
        match_case_id: ast::NodeId<ast::MatchCase>,
        tree: &mut NodeTree,
    ) -> NodeId<MatchCase> {
        let (symbol_id, scope_id) =
            tree.create_symbol_with_scope(SymbolSpace::Value, None, ScopeKind::Block, scope_id);
        let match_case = module.get(match_case_id);
        let match_case = match match_case {
            ast::MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree);
                let body = self.bind_expression(module, scope_id, *body, tree);
                let guard = guard.map(|guard| self.bind_expression(module, scope_id, guard, tree));
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
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree);
                let body = self.bind_block(module, scope_id, *body, tree);
                let guard = guard.map(|guard| self.bind_expression(module, scope_id, guard, tree));
                MatchCase::Block {
                    pattern,
                    body,
                    guard,
                    scope: scope_id,
                }
            }
        };
        tree.insert_from_source_as_symbol(match_case, module.id, match_case_id, symbol_id)
    }
}
