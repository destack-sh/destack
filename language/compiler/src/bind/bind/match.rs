use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::{BindState, BindingContext};

use crate::Compiler;

impl Compiler {
    /// Bind one match case inside its case scope.
    pub(in crate::bind) fn bind_match_case(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        state.bind_node(id.into_any());

        match match_case {
            dir::MatchCase::Expression { selector, body } => {
                self.bind_match_selector(state, tree, selector);
                let body_id = *body;
                let body = tree.get(body_id);
                state.visit_expression(tree, body_id, body);
            }
            dir::MatchCase::Block { selector, body } => {
                self.bind_match_selector(state, tree, selector);
                let body_id = *body;
                let body = tree.get(body_id);
                state.visit_block(tree, body_id, body);
            }
        }
    }

    /// Bind one match selector pattern.
    fn bind_match_selector(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        selector: &dir::MatchSelector,
    ) {
        let dir::MatchSelector::Pattern { pattern, guard } = selector else {
            return;
        };

        // bind selected pattern without inheriting declaration context
        let pattern_node = tree.get(*pattern);
        state.push_binding(BindingContext::default());
        state.visit_pattern(tree, *pattern, pattern_node);
        state.pop_binding();

        // visit pattern guard
        if let Some(guard_expr) = guard {
            let guard = tree.get(*guard_expr);
            state.visit_expression(tree, *guard_expr, guard);
        }
    }
}
