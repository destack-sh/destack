use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{CheckModuleState, PatternRelation, VariableId};

use super::expression::ConditionBranch;

impl CheckModuleState {
    /// Walk one match case.
    pub(in crate::check) fn walk_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
        value: Option<VariableId>,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::MatchCase, id.id);

        match match_case {
            // case pattern if guard => expression
            dir::MatchCase::Expression { selector, body } => {
                // enter selector facts before the body
                self.walk_match_selector(tree, selector, value);

                self.walk_expression(tree, *body, tree.get(*body));
            }
            // case pattern if guard { ... }
            dir::MatchCase::Block { selector, body } => {
                // enter selector facts before the body
                self.walk_match_selector(tree, selector, value);

                self.walk_block(tree, *body, tree.get(*body));
            }
        }
    }

    /// Walk one match selector into arm-local flow state.
    fn walk_match_selector(
        &mut self,
        tree: &dir::Tree,
        selector: &dir::MatchSelector,
        value: Option<VariableId>,
    ) {
        match selector {
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                // walk pattern and relate it to the matched value
                self.walk_pattern(tree, *pattern, tree.get(*pattern));

                if let Some(value) = value
                    && let Some(term) = self.pattern_term(*pattern, tree)
                {
                    self.relate_pattern(PatternRelation::Match(term), pattern.into_any(), value);
                }

                // pattern bindings are assigned in the selected arm
                self.mark_bindings_assigned(tree, pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    self.walk_expression(tree, *guard, tree.get(*guard));

                    let variable = self.intern_local_type_variable(*guard);
                    self.constrain_condition(guard.into_any(), variable);
                    self.apply_expression_narrowings(tree, *guard, ConditionBranch::True);
                }
            }
            // default
            dir::MatchSelector::Default => {}
        }
    }
}
