use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    ConditionBranch, Expectation, ExpectedType, FlowPath, FlowPredicate, MatchCase, Obligation,
    Origin, PatternCoverage, PatternCoverageObligation, Relation, StaticGate, ValueUse, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one match expression with isolated case flow.
    ///
    /// Example:
    /// ```ds
    /// match value { case Some(item) => item }
    /// ```
    pub(in crate::check) fn walk_match_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<()> {
        let (value_expectation, value_path) = self.walk_match_scrutinee(value)?;
        let active_cases = self.active_match_cases(cases)?;
        let coverage_cases =
            self.walk_active_match_cases(value, &active_cases, &value_expectation, &value_path)?;

        self.queue_match_coverage(id, value_expectation, coverage_cases);

        Ok(())
    }

    /// Walk one match scrutinee and return its pattern input.
    fn walk_match_scrutinee(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(ExpectedType, Option<FlowPath>)> {
        self.walk_expression(value, self.tree.get(value))?;

        let value_site = self.node_site(value)?;
        let value_expectation = ExpectedType::Node(value_site);
        let value_path = self.flow_path(value);

        Ok((value_expectation, value_path))
    }

    /// Return match cases included by their static gates.
    fn active_match_cases(
        &mut self,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::MatchCase>>> {
        let mut active_cases = Vec::new();
        for case in cases {
            match self.decorated_static_gate(case.into_any())? {
                StaticGate::Absent => {}
                StaticGate::Present => active_cases.push(*case),
            }
        }

        Ok(active_cases)
    }

    /// Walk present match cases with isolated branch flow.
    fn walk_active_match_cases(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
        value_expectation: &ExpectedType,
        value_path: &Option<FlowPath>,
    ) -> CompilerResult<Vec<MatchCase>> {
        let before = self.fork_flow();
        let mut coverage_cases = Vec::new();
        let mut excluded_patterns = Vec::new();
        let mut merged = None;

        for case in cases {
            // replay exclusions from previous cases
            self.restore_flow(before);
            if let Some(path) = value_path {
                for pattern in &excluded_patterns {
                    self.exclude_match_pattern(path.clone(), *pattern);
                }
            }

            // walk the arm under the current narrowed scrutinee
            let expected = match value_path {
                Some(_) => ExpectedType::Node(self.node_site(value)?),
                None => *value_expectation,
            };
            self.walk_match_case(
                *case,
                self.tree.get(*case),
                Some((expected, value_path.clone())),
            )?;

            // record the arm for exhaustiveness and later exclusions
            coverage_cases.extend(self.match_case_coverage(*case)?);
            if let Some(pattern) = self.match_case_exclusion_pattern(*case) {
                excluded_patterns.push(pattern);
            }

            // merge completing branches into the post-match flow
            if self.match_case_can_complete_normally(self.tree.get(*case)) {
                let case_flow = self.collect_flow_branch(before);
                merged = match merged.take() {
                    Some(previous) => {
                        self.merge_flow_branches(before, &previous, &case_flow);

                        Some(self.collect_flow_branch(before))
                    }
                    None => Some(case_flow),
                };
            }
        }

        // restore the merged branch output, or the pre-match input if no case completes
        if let Some(merged) = merged {
            self.restore_flow_branch(before, &merged);
        } else {
            self.restore_flow(before);
        }

        Ok(coverage_cases)
    }

    /// Walk one match case.
    ///
    /// Example:
    /// ```ds
    /// case Some(value) if value > 0 => value
    /// ```
    pub(in crate::check) fn walk_match_case(
        &mut self,
        _id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
        value: Option<(ExpectedType, Option<FlowPath>)>,
    ) -> CompilerResult<()> {
        match match_case {
            // case pattern if guard => expression
            dir::MatchCase::Expression { selector, body } => {
                // enter selector flow before the body
                self.walk_match_selector(selector, value)?;

                self.walk_expression(*body, self.tree.get(*body))?;
            }
            // case pattern if guard { ... }
            dir::MatchCase::Block { selector, body } => {
                // enter selector flow before the body
                self.walk_match_selector(selector, value)?;

                self.walk_block(*body, self.tree.get(*body))?;
            }
        };

        Ok(())
    }

    /// Walk one match selector into arm-local flow state.
    ///
    /// Example:
    /// ```ds
    /// case Some(value) if value > 0
    /// ```
    fn walk_match_selector(
        &mut self,
        selector: &dir::MatchSelector,
        value: Option<(ExpectedType, Option<FlowPath>)>,
    ) -> CompilerResult<()> {
        match selector {
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                // constrain pattern type from the matched value
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;

                if let Some((expected, path)) = value {
                    let expectation = Expectation {
                        expected,
                        relation: Relation::Assignable,
                        origin: Origin::Node(
                            pattern.into_global_any(self.module),
                            self.flow().template_scope(),
                        ),
                        use_: ValueUse::Store,
                    };
                    self.queue_node_check(*pattern, expectation)?;

                    if let Some(path) = path {
                        self.narrow_pattern(path, *pattern, true)?;
                    }
                }

                // pattern bindings are assigned in the matching arm
                self.mark_bindings_assigned(pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    let expectation = self.condition_expectation(*guard)?;
                    self.walk_expression(*guard, self.tree.get(*guard))?;
                    self.queue_node_check(*guard, expectation)?;
                    self.narrow_expression(*guard, ConditionBranch::True)?;
                }
            }
            // default
            dir::MatchSelector::Default => {}
        };

        Ok(())
    }

    /// Queue coverage checking for one match expression.
    fn queue_match_coverage(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: ExpectedType,
        cases: Vec<MatchCase>,
    ) {
        self.check.push_obligation(
            Obligation::PatternCoverage(PatternCoverageObligation {
                source: id.into_global_any(self.module),
                value,
                coverage: PatternCoverage::Match { cases },
            }),
            self.flow().template_scope(),
        );
    }

    /// Return the coverage case for one match arm.
    fn match_case_coverage(
        &mut self,
        case: dir::LocalNodeId<dir::MatchCase>,
    ) -> CompilerResult<Option<MatchCase>> {
        let selector = match self.tree.get(case) {
            dir::MatchCase::Expression { selector, .. }
            | dir::MatchCase::Block { selector, .. } => selector,
        };

        match selector {
            // default
            dir::MatchSelector::Default => Ok(Some(MatchCase::Default)),
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                let (pattern, guard) = (*pattern, *guard);

                Ok(Some(MatchCase::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_guarded: guard.is_some(),
                }))
            }
        }
    }

    /// Return the unguarded pattern that later match arms can exclude.
    pub(in crate::check) fn match_case_exclusion_pattern(
        &self,
        case: dir::LocalNodeId<dir::MatchCase>,
    ) -> Option<dir::GlobalNodeId<dir::Pattern>> {
        let selector = match self.tree.get(case) {
            dir::MatchCase::Expression { selector, .. }
            | dir::MatchCase::Block { selector, .. } => selector,
        };

        match selector {
            // unguarded patterns remove matched values from later arms
            dir::MatchSelector::Pattern {
                pattern,
                guard: None,
            } => Some(pattern.into_global(self.module)),
            // guarded arms and defaults leave later arms unchanged
            dir::MatchSelector::Pattern { guard: Some(_), .. } | dir::MatchSelector::Default => {
                None
            }
        }
    }

    /// Exclude one previously matched pattern from a flow path.
    pub(in crate::check) fn exclude_match_pattern(
        &mut self,
        path: FlowPath,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) {
        let predicate = FlowPredicate::Pattern {
            pattern,
            is_positive: false,
        };

        self.apply_flow_predicate(path, predicate);
    }
}
