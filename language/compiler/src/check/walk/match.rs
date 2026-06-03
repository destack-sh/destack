use destack_dir as dir;

use crate::check::{FlowPath, PatternRelation, TypeOperand, WalkState};

use super::expression::ConditionBranch;

impl WalkState<'_, '_> {
    /// Walk one match case.
    ///
    /// Example:
    /// ```ds
    /// case Some(value) if value > 0 => value
    /// ```
    pub(in crate::check) fn walk_match_case(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
        value: Option<(TypeOperand, Option<FlowPath>)>,
    ) {
        match match_case {
            // case pattern if guard => expression
            dir::MatchCase::Expression { selector, body } => {
                // enter selector flow before the body
                self.walk_match_selector(tree, selector, value);

                self.walk_expression(tree, *body, tree.get(*body));
            }
            // case pattern if guard { ... }
            dir::MatchCase::Block { selector, body } => {
                // enter selector flow before the body
                self.walk_match_selector(tree, selector, value);

                self.walk_block(tree, *body, tree.get(*body));
            }
        };
    }

    /// Walk one match selector into arm-local flow state.
    ///
    /// Example:
    /// ```ds
    /// case Some(value) if value > 0
    /// ```
    fn walk_match_selector(
        &mut self,
        tree: &dir::Tree,
        selector: &dir::MatchSelector,
        value: Option<(TypeOperand, Option<FlowPath>)>,
    ) {
        match selector {
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                // walk pattern and constrain it against the matched value
                self.walk_pattern(tree, *pattern, tree.get(*pattern));

                if let Some((value, path)) = value
                    && let Some(term) = self.lower_pattern_term(tree.module_id, *pattern, tree)
                {
                    let condition = self.active_static_guard();

                    self.check.relate_pattern(
                        tree.module_id,
                        PatternRelation::Match(term),
                        pattern.into_any(),
                        value,
                        condition,
                    );

                    if let Some(path) = path {
                        self.narrow_pattern_success(tree, path, *pattern);
                    }
                }

                // pattern bindings are assigned in the selected arm
                self.mark_bindings_assigned(tree, pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    self.walk_expression(tree, *guard, tree.get(*guard));

                    let variable = self.allocate_node_type_operand(*guard);
                    let condition = self.active_static_guard();

                    self.check.constrain_condition(
                        tree.module_id,
                        guard.into_any(),
                        variable,
                        condition,
                    );
                    self.narrow_expression(tree, *guard, ConditionBranch::True);
                }
            }
            // default
            dir::MatchSelector::Default => {}
        };
    }
}
