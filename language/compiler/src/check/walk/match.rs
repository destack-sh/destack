use destack_dir as dir;

use crate::check::{CheckState, FlowPath, MatchCase, PatternRelation, VariableId};

use super::expression::ConditionBranch;

impl CheckState<'_> {
    /// Walk one match case.
    pub(in crate::check) fn walk_match_case(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
        value: Option<(VariableId, Option<FlowPath>)>,
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
    fn walk_match_selector(
        &mut self,
        tree: &dir::Tree,
        selector: &dir::MatchSelector,
        value: Option<(VariableId, Option<FlowPath>)>,
    ) {
        match selector {
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                // walk pattern and constrain it against the matched value
                self.walk_pattern(tree, *pattern, tree.get(*pattern));

                if let Some((value, path)) = value
                    && let Some(term) = self.build_pattern_term(tree.module_id, *pattern, tree)
                {
                    let condition = self.active_static_condition(tree.module_id);

                    self.constrain_pattern(
                        tree.module_id,
                        PatternRelation::Match(term),
                        pattern.into_any(),
                        value,
                        condition,
                    );

                    if let Some(path) = path {
                        self.apply_pattern_success_narrowings(tree, path, *pattern);
                    }
                }

                // pattern bindings are assigned in the selected arm
                self.mark_bindings_assigned(tree, pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    self.walk_expression(tree, *guard, tree.get(*guard));

                    let variable = self.intern_local_node_type_variable(tree.module_id, *guard);
                    let condition = self.active_static_condition(tree.module_id);

                    self.constrain_condition(guard.into_any(), variable, condition);
                    self.apply_expression_narrowings(tree, *guard, ConditionBranch::True);
                }
            }
            // default
            dir::MatchSelector::Default => {}
        };
    }

    /// Return match case terms for active cases.
    pub(in crate::check) fn build_match_case_terms(
        &mut self,
        tree: &dir::Tree,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> Vec<MatchCase> {
        cases
            .iter()
            .filter_map(|case| self.build_match_case_term(tree, tree.get(*case)))
            .collect()
    }

    /// Return one match case term.
    fn build_match_case_term(
        &mut self,
        tree: &dir::Tree,
        match_case: &dir::MatchCase,
    ) -> Option<MatchCase> {
        self.build_match_selector_term(tree, match_case.selector())
    }

    /// Return one match selector term.
    fn build_match_selector_term(
        &mut self,
        tree: &dir::Tree,
        selector: &dir::MatchSelector,
    ) -> Option<MatchCase> {
        let term = match selector {
            dir::MatchSelector::Default => MatchCase::Default,
            dir::MatchSelector::Pattern { pattern, guard } => MatchCase::PatternTerm {
                pattern: self.build_pattern_term(tree.module_id, *pattern, tree)?,
                guard: guard
                    .map(|guard| self.intern_local_node_type_variable(tree.module_id, guard)),
            },
        };

        Some(term)
    }
}
