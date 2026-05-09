use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR MatchSelector to an AST MatchSelector.
    fn unbind_match_selector(
        &self,
        module: &Module,
        selector: &dir::MatchSelector,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::MatchSelector {
        match selector {
            dir::MatchSelector::Pattern { pattern, guard } => {
                let ast_pattern = self.unbind_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let ast_guard = guard.map(|guard| {
                    self.unbind_expression(
                        module,
                        guard,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
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
        match_form: ast::MatchForm,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
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
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body = self.unbind_expression(
                    module,
                    *body,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::MatchCase::Expression { selector, body }
            }
            dir::MatchCase::Block { selector, body, .. } => {
                let selector = self.unbind_match_selector(
                    module,
                    selector,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body_context = match match_form {
                    ast::MatchForm::Match => ast::BlockContext::Expression,
                    ast::MatchForm::Switch => ast::BlockContext::Statement,
                };
                let body = self.unbind_block(
                    module,
                    *body,
                    body_context,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::MatchCase::Block { selector, body }
            }
        };
        let ast_case_id = ast_tree.insert(ast_case, span);
        context.map(case_id.into_any(), ast_case_id.into_any());
        ast_case_id
    }
}
