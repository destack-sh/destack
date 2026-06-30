use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    ConditionBranch, Expectation, ExpectedType, FlowNarrowing, FlowPath, Origin, Relation,
    ValueUse, WalkState,
};

impl WalkState<'_, '_> {
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

                self.walk_expression(*body, self.tree.get(*body), None)?;
            }
            // case pattern if guard { ... }
            dir::MatchCase::Block { selector, body } => {
                // enter selector flow before the body
                self.walk_match_selector(selector, value)?;

                self.walk_block(*body, self.tree.get(*body), None)?;
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
                        origin: Origin::Node(pattern.into_global_any(self.module)),
                        use_: ValueUse::Store,
                    };
                    self.queue_node_check(*pattern, expectation);

                    if let Some(path) = path {
                        self.narrow_pattern_match(path, *pattern)?;
                    }
                }

                // pattern bindings are assigned in the selected arm
                self.mark_bindings_assigned(pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    let expectation = self.condition_expectation(*guard)?;
                    self.walk_expression(*guard, self.tree.get(*guard), Some(&expectation))?;
                    self.narrow_expression(*guard, ConditionBranch::True)?;
                }
            }
            // default
            dir::MatchSelector::Default => {}
        };

        Ok(())
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
        let narrowing = FlowNarrowing::Pattern {
            pattern,
            is_positive: false,
        };

        self.narrow_flow_path(path, narrowing);
    }
}
