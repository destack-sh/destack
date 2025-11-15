use dyst_ast::{self as ast};
use dyst_dir::{MatchCase, Module, NodeId};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a match case to a DIR match case.
    pub fn lower_match_case(
        &mut self,
        module: &Module,
        match_case_id: ast::NodeId<ast::MatchCase>,
    ) -> NodeId<MatchCase> {
        let match_case = module.get(match_case_id);
        let match_case = match match_case {
            ast::MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.lower_pattern(module, *pattern);
                let body = self.lower_expression(module, *body);
                let guard = guard.map(|guard| self.lower_expression(module, guard));
                MatchCase::Expression {
                    pattern,
                    body,
                    guard,
                }
            }
            ast::MatchCase::Block {
                pattern,
                body,
                guard,
            } => {
                let pattern = self.lower_pattern(module, *pattern);
                let body = self.lower_block(module, *body);
                let guard = guard.map(|guard| self.lower_expression(module, guard));
                MatchCase::Block {
                    pattern,
                    body,
                    guard,
                }
            }
        };
        self.session
            .tree
            .insert_from_ast(match_case, module.id, match_case_id)
    }
}
