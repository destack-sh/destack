use destack_dir as dir;

use crate::check::{
    ConditionBranch, ControlTargetForm, ExpectedType, FlowBranch, FlowPath, FlowPredicate,
    Obligation, PatternArm, PatternCoverage, PatternCoverageObligation, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one match expression with isolated arm flow.
    pub(in crate::check) fn walk_match_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<()> {
        let (value_expectation, value_path) = self.walk_selected_value(value)?;
        let arms = self.present_match_arms(arms)?;
        let coverage = self.walk_match_arms(&arms, &value_path)?;
        self.queue_match_coverage(id, value_expectation, coverage);

        Ok(())
    }

    /// Walk one switch statement with ordered selection and fallthrough.
    pub(in crate::check) fn walk_switch_statement(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<()> {
        let (_, value_path) = self.walk_selected_value(value)?;
        let cases = self.present_switch_cases(cases)?;
        let before = self.fork_flow();
        self.enter_control_target(None, ControlTargetForm::Switch);

        // evaluate selectors in source order and retain each equality branch
        let mut direct = vec![None; cases.len()];
        let mut default_index = None;
        for (index, case) in cases.iter().enumerate() {
            let selector = self.tree.get(*case).selector;
            match selector {
                dir::SwitchSelector::Case(selector) => {
                    self.walk_expression(selector, self.tree.get(selector))?;
                    let after_selector = self.fork_flow();

                    if let Some(path) = &value_path {
                        self.apply_switch_equality(path.clone(), selector, true);
                    }
                    direct[index] = Some(self.collect_flow_branch(before));

                    self.restore_flow(after_selector);
                    if let Some(path) = &value_path {
                        self.apply_switch_equality(path.clone(), selector, false);
                    }
                }
                dir::SwitchSelector::Default => default_index = Some(index),
            }
        }
        let unmatched = self.collect_flow_branch(before);
        let unmatched = match default_index {
            Some(index) => {
                direct[index] = Some(unmatched);

                None
            }
            None => Some(unmatched),
        };

        // execute bodies in source order and join adjacent fallthrough
        let mut fallthrough: Option<FlowBranch> = None;
        for (index, case) in cases.iter().enumerate() {
            let Some(selected) = direct[index].as_ref() else {
                return Err(CompilerError::Internal {
                    message: format!("switch case {case:?} has no selection entry"),
                });
            };
            if let Some(previous) = &fallthrough {
                self.merge_flow_branches(before, selected, previous);
            } else {
                self.restore_flow_branch(before, selected);
            }

            let body = self.tree.get(*case).body;
            self.walk_block(body, self.tree.get(body))?;
            fallthrough = self
                .block_can_complete_normally(self.tree.get(body))
                .then(|| self.collect_flow_branch(before));
        }

        // join explicit breaks, final fallthrough, and an unmatched value
        let mut exits = self.leave_control_target();
        exits.extend(fallthrough);
        exits.extend(unmatched);
        if exits.is_empty() {
            self.check
                .module_mut(self.module)
                .unreachable_ends
                .insert(id.into_any());
        }
        self.merge_flow_branches_from(before, &exits);

        Ok(())
    }

    /// Walk one selected value and return its pattern input.
    fn walk_selected_value(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(ExpectedType, Option<FlowPath>)> {
        self.walk_expression(value, self.tree.get(value))?;

        let expected = ExpectedType::Node(value.into_global_any(self.module));
        let path = self.flow_path(value);

        Ok((expected, path))
    }

    /// Return match arms included by their static gates.
    fn present_match_arms(
        &mut self,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::MatchArm>>> {
        let mut present = Vec::new();
        for arm in arms {
            if self.walk_decorators(arm.into_any())? {
                self.enter_node(*arm)?;
                present.push(*arm);
            }
        }

        Ok(present)
    }

    /// Return switch cases included by their static gates.
    fn present_switch_cases(
        &mut self,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::SwitchCase>>> {
        let mut present = Vec::new();
        for case in cases {
            if self.walk_decorators(case.into_any())? {
                self.enter_node(*case)?;
                present.push(*case);
            }
        }

        Ok(present)
    }

    /// Walk present match arms with isolated branch flow.
    fn walk_match_arms(
        &mut self,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        value_path: &Option<FlowPath>,
    ) -> CompilerResult<Vec<PatternArm>> {
        let before = self.fork_flow();
        let mut coverage = Vec::new();
        let mut excluded = Vec::new();
        let mut merged = None;

        for arm in arms {
            // replay exclusions from previous arms
            self.restore_flow(before);
            if let Some(path) = value_path {
                for pattern in &excluded {
                    self.exclude_match_pattern(path.clone(), *pattern);
                }
            }

            // walk this arm under the narrowed selected value
            let arm_node = self.tree.get(*arm).clone();
            self.walk_match_arm(&arm_node, value_path.clone())?;
            coverage.push(self.match_arm_coverage(*arm));
            if let Some(pattern) = self.match_arm_exclusion_pattern(*arm) {
                excluded.push(pattern);
            }

            // merge completing arms into the post-match flow
            if self.match_arm_can_complete_normally(&arm_node) {
                let branch = self.collect_flow_branch(before);
                merged = match merged.take() {
                    Some(previous) => {
                        self.merge_flow_branches(before, &previous, &branch);

                        Some(self.collect_flow_branch(before))
                    }
                    None => Some(branch),
                };
            }
        }

        // restore the merged output or the pre-match input
        if let Some(merged) = merged {
            self.restore_flow_branch(before, &merged);
        } else {
            self.restore_flow(before);
        }

        Ok(coverage)
    }

    /// Walk one match arm.
    fn walk_match_arm(
        &mut self,
        arm: &dir::MatchArm,
        value_path: Option<FlowPath>,
    ) -> CompilerResult<()> {
        let pattern = arm.pattern();
        self.walk_pattern(pattern, self.tree.get(pattern), None)?;
        if let Some(path) = value_path {
            self.narrow_pattern(path, pattern, true)?;
        }
        self.mark_bindings_assigned(pattern.into_any());

        // apply the optional arm guard
        if let Some(guard) = arm.guard() {
            self.walk_expression(guard, self.tree.get(guard))?;
            self.narrow_expression(guard, ConditionBranch::True)?;
        }

        // walk the selected body
        match arm {
            dir::MatchArm::Expression { body, .. } => {
                self.walk_expression(*body, self.tree.get(*body))?;
            }
            dir::MatchArm::Block { body, .. } => {
                self.walk_block(*body, self.tree.get(*body))?;
            }
        }

        Ok(())
    }

    /// Queue coverage checking for one match expression.
    fn queue_match_coverage(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: ExpectedType,
        arms: Vec<PatternArm>,
    ) {
        self.check.push_obligation(
            Obligation::PatternCoverage(PatternCoverageObligation {
                source: id.into_global_any(self.module),
                value,
                coverage: PatternCoverage::Match { arms },
            }),
            self.flow().template_scope(),
        );
    }

    /// Return the coverage case for one match arm.
    fn match_arm_coverage(&self, arm: dir::LocalNodeId<dir::MatchArm>) -> PatternArm {
        let arm = self.tree.get(arm);

        PatternArm {
            pattern: arm.pattern().into_global(self.module),
            is_guarded: arm.guard().is_some(),
        }
    }

    /// Return the unguarded pattern excluded from later match arms.
    fn match_arm_exclusion_pattern(
        &self,
        arm: dir::LocalNodeId<dir::MatchArm>,
    ) -> Option<dir::GlobalNodeId<dir::Pattern>> {
        let arm = self.tree.get(arm);

        arm.guard()
            .is_none()
            .then(|| arm.pattern().into_global(self.module))
    }

    /// Apply one switch equality branch to a stable selected path.
    fn apply_switch_equality(
        &mut self,
        path: FlowPath,
        value: dir::LocalNodeId<dir::Expression>,
        is_positive: bool,
    ) {
        let predicate = FlowPredicate::Equality {
            value: value.into_global(self.module),
            is_positive,
        };
        self.apply_flow_predicate(path, predicate);
    }

    /// Exclude one previously matched pattern from a flow path.
    fn exclude_match_pattern(&mut self, path: FlowPath, pattern: dir::GlobalNodeId<dir::Pattern>) {
        let predicate = FlowPredicate::Pattern {
            pattern,
            is_positive: false,
        };
        self.apply_flow_predicate(path, predicate);
    }
}
