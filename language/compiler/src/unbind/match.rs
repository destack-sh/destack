use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR match case to an AST match case.
    pub(super) fn unbind_match_case(
        &self,
        module: &Module,
        case_id: dir::LocalNodeId<dir::MatchCase>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::LocalNodeId<ast::MatchCase> {
        let case = tree.get(case_id);
        let span = self.unbind_span(module, case_id.into());
        let ast_case = match case {
            dir::MatchCase::Expression {
                pattern,
                body,
                guard,
                ..
            } => {
                let pattern =
                    self.unbind_pattern(module, *pattern, tree, symbols, ast_tree, ast_strings);
                let body =
                    self.unbind_expression(module, *body, tree, symbols, ast_tree, ast_strings);
                let guard = guard.map(|guard| {
                    self.unbind_expression(module, guard, tree, symbols, ast_tree, ast_strings)
                });
                ast::MatchCase::Expression {
                    pattern,
                    body,
                    guard,
                }
            }
            dir::MatchCase::Block {
                pattern,
                body,
                guard,
                ..
            } => {
                let pattern =
                    self.unbind_pattern(module, *pattern, tree, symbols, ast_tree, ast_strings);
                let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings);
                let guard = guard.map(|guard| {
                    self.unbind_expression(module, guard, tree, symbols, ast_tree, ast_strings)
                });
                ast::MatchCase::Block {
                    pattern,
                    body,
                    guard,
                }
            }
        };
        ast_tree.insert(ast_case, span)
    }
}
