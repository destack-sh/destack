use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{ConditionBranch, FlowPath, Origin, Relation, WalkState};

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
        value: Option<(dir::GlobalTypeId, Option<FlowPath>)>,
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
        value: Option<(dir::GlobalTypeId, Option<FlowPath>)>,
    ) -> CompilerResult<()> {
        match selector {
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                // walk pattern and flow the matched value into its holes
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;

                if let Some((value, path)) = value {
                    let origin = Origin::Node(pattern.into_global_any(self.module));
                    let pattern_type = self.node_type(*pattern)?;
                    self.relate_type(origin, Relation::Assignable, value, pattern_type);

                    if let Some(path) = path {
                        self.narrow_pattern_match(path, *pattern)?;
                    }
                }

                // pattern bindings are assigned in the selected arm
                self.mark_bindings_assigned(pattern.into_any());

                // if guard
                if let Some(guard) = guard {
                    self.walk_expression(*guard, self.tree.get(*guard))?;

                    // guards produce runtime booleans
                    let origin = Origin::Node(guard.into_global_any(self.module));
                    let condition = self.node_type(*guard)?;
                    let boolean = self.push_type(
                        dir::Type::Primitive(dir::PrimitiveType::Boolean),
                        guard.into_any(),
                    )?;
                    self.relate_type(origin, Relation::Assignable, condition, boolean);
                    self.narrow_expression(*guard, ConditionBranch::True)?;
                }
            }
            // default
            dir::MatchSelector::Default => {}
        };

        Ok(())
    }
}
