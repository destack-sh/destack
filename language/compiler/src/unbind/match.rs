use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR MatchSelector to an AST MatchSelector.
    fn unbind_match_selector(
        &self,
        module: &Module,
        selector: &dir::MatchSelector,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::MatchSelector {
        match selector {
            dir::MatchSelector::Pattern { pattern, guard } => {
                let ast_pattern =
                    self.unbind_pattern(module, *pattern, tree, symbols, ast_tree, ast_strings);
                let ast_guard = guard.map(|guard| {
                    self.unbind_expression(module, guard, tree, symbols, ast_tree, ast_strings)
                });
                ast::MatchSelector::Pattern {
                    pattern: ast_pattern,
                    guard: ast_guard,
                }
            }
            dir::MatchSelector::Default => ast::MatchSelector::Default,
        }
    }

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
            dir::MatchCase::Expression { selector, body, .. } => {
                let selector = self.unbind_match_selector(
                    module,
                    selector,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                );
                let body =
                    self.unbind_expression(module, *body, tree, symbols, ast_tree, ast_strings);
                ast::MatchCase::Expression { selector, body }
            }
            dir::MatchCase::Block { selector, body, .. } => {
                let selector = self.unbind_match_selector(
                    module,
                    selector,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                );
                let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings);
                ast::MatchCase::Block { selector, body }
            }
        };
        ast_tree.insert(ast_case, span)
    }
}
