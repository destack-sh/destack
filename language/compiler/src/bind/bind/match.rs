use dir::NodeVisitor as _;
use tspp_dir as dir;

use super::super::state::{BindState, BindingModifiers};

use crate::Compiler;

impl Compiler {
    /// Bind one match arm inside its arm scope.
    pub(in crate::bind) fn bind_match_arm(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchArm>,
        arm: &dir::MatchArm,
    ) {
        state.bind_node(id.into_any());
        let pattern = arm.pattern();

        // bind immutable names selected by the arm pattern
        let pattern_node = tree.get(pattern);
        state.push_binding_modifiers(BindingModifiers {
            export: None,
            mutability: Some(dir::Mutability::Immutable),
            is_shared: false,
            kind: dir::SymbolKind::Variable,
        });
        state.visit_pattern(tree, pattern, pattern_node);
        state.pop_binding_modifiers();

        // bind the optional guard
        if let Some(guard) = arm.guard() {
            self.bind_condition_operands(state, tree, &guard.operands);
        }

        // bind the arm body
        match arm {
            dir::MatchArm::Expression { body, .. } => {
                let expression = tree.get(*body);
                state.visit_expression(tree, *body, expression);
            }
            dir::MatchArm::Block { body, .. } => {
                let block = tree.get(*body);
                state.visit_block(tree, *body, block);
            }
        }
    }
}
