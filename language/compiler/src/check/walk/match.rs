use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{ConditionBranch, FlowPath, PatternRelation, TypeOperand, WalkState};

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
        value: Option<(TypeOperand, Option<FlowPath>)>,
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
        value: Option<(TypeOperand, Option<FlowPath>)>,
    ) -> CompilerResult<()> {
        match selector {
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                // walk pattern and constrain it against the matched value
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;

                let term = self.pattern_term(self.module, *pattern)?;

                if let (Some((value, path)), Some(term)) = (value, term) {
                    let condition = self.active_static_guard();

                    self.check.relate_pattern(
                        self.module,
                        PatternRelation::Match(term),
                        pattern.into_any(),
                        value,
                        condition,
                    );

                    if let Some(path) = path {
                        self.narrow_pattern_success(path, *pattern)?;
                    }
                }

                // pattern bindings are assigned in the selected arm
                self.mark_bindings_assigned(pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    self.walk_expression(*guard, self.tree.get(*guard))?;

                    let variable = self.node_type_operand(*guard)?;
                    let condition = self.active_static_guard();

                    self.check.constrain_condition(
                        self.module,
                        guard.into_any(),
                        variable,
                        condition,
                    );
                    self.narrow_expression(*guard, ConditionBranch::True)?;
                }
            }
            // default
            dir::MatchSelector::Default => {}
        };

        Ok(())
    }
}
