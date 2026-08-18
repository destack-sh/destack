use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

/// One authored for-of expression.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ForOf<'a> {
    /// Whether iteration awaits each value.
    pub(crate) asynchrony: dir::Asynchrony,
    /// The element binding.
    pub(crate) binding: &'a dir::ForEachBinding,
    /// The iterated expression.
    pub(crate) iterator: dir::LocalNodeId<dir::Expression>,
    /// The loop body.
    pub(crate) body: dir::LocalNodeId<dir::Block>,
}

impl DirModule<'_> {
    /// Return one unguarded match arm that contains exactly one expression.
    pub(crate) fn match_arm_expression(
        &self,
        arm: dir::LocalNodeId<dir::MatchArm>,
    ) -> Option<(
        dir::LocalNodeId<dir::Pattern>,
        dir::LocalNodeId<dir::Expression>,
    )> {
        let view = self.view();
        let arm = view.get(arm);
        if arm.guard().is_some() {
            return None;
        }

        // select the direct body or the only expression in its block
        let body = match arm {
            dir::MatchArm::Expression { body, .. } => Some(*body),
            dir::MatchArm::Block { body, .. } => view.get(*body).only_expression(),
        }?;

        Some((arm.pattern(), body))
    }

    /// Return one unguarded match arm that consists only of a value.
    pub(crate) fn match_arm_value(
        &self,
        arm: dir::LocalNodeId<dir::MatchArm>,
    ) -> Option<(
        dir::LocalNodeId<dir::Pattern>,
        dir::LocalNodeId<dir::Expression>,
    )> {
        let view = self.view();
        let arm = view.get(arm);
        if arm.guard().is_some() {
            return None;
        }

        // select the direct body or an unpreceded block value
        let body = match arm {
            dir::MatchArm::Expression { body, .. } => Some(*body),
            dir::MatchArm::Block { body, .. } => {
                let body = view.get(*body);
                if body.leading_expressions.is_empty() {
                    body.value_expression()
                } else {
                    None
                }
            }
        }?;

        Some((arm.pattern(), body))
    }

    /// Return one authored for-of expression.
    pub(crate) fn for_of(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ForOf<'_>> {
        // select the shared for-each variant by its operator
        let view = self.view();
        let dir::Expression::ForEach {
            asynchrony,
            operator: dir::ForEachOperator::Of,
            binding,
            iterator,
            body,
            ..
        } = view.get(expression)
        else {
            return None;
        };

        Some(ForOf {
            asynchrony: *asynchrony,
            binding,
            iterator: *iterator,
            body: *body,
        })
    }

    /// Return the body of one authored iteration expression.
    pub(crate) fn iteration_body(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Block>> {
        match self.view().get(expression) {
            dir::Expression::While { body, .. }
            | dir::Expression::ForEach { body, .. }
            | dir::Expression::For { body, .. }
            | dir::Expression::Loop { body, .. } => Some(*body),
            _ => None,
        }
    }

    /// Return the target selected by one unlabeled break or continue expression.
    pub(crate) fn unlabeled_transfer_target(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let transfer = view.get(expression);
        if !matches!(
            transfer,
            dir::Expression::Break { .. } | dir::Expression::Continue { .. }
        ) {
            return None;
        }
        let mut parent = view.get_parent_for(expression);

        // climb to the nearest compatible target or callable boundary
        while let Some(node) = parent {
            // stop at nested callable ownership
            if self.callable_body(node).is_some() {
                return None;
            }

            // select the nearest iteration or compatible switch
            if let Ok(expression) = node.try_into_typed::<dir::Expression>() {
                let is_target = self.iteration_body(expression).is_some()
                    || matches!(transfer, dir::Expression::Break { .. })
                        && matches!(view.get(expression), dir::Expression::Switch { .. });
                if is_target {
                    return Some(expression);
                }
            }

            parent = view.get_parent_any(node);
        }

        None
    }

    /// Return the checked control target selected by one break or continue expression.
    pub(crate) fn transfer_target(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::LocalNodeId<dir::Expression>, ProviderError> {
        // read the checked target selected for this transfer
        let global = expression.into_global_any(self.id);
        let target = self.decisions.transfer_decision(global).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked control transfer {global:?} has no target decision"
            ))
        })?;

        // require a target in the linted module
        if target.module_id != self.id {
            return Err(ProviderError::internal(format!(
                "control target {target:?} belongs to another module"
            )));
        }

        Ok(target.local_id)
    }

    /// Return whether one reachable continue selects an iteration expression.
    pub(crate) fn has_reachable_continue(
        &self,
        iteration: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // require an iteration target
        if self.iteration_body(iteration).is_none() {
            let iteration = iteration.into_global_any(self.id);

            return Err(ProviderError::internal(format!(
                "reachable continue query target {iteration:?} is not an iteration expression"
            )));
        }

        let target = iteration.into_global(self.id);
        let view = self.view();

        // inspect checked transfers selecting this iteration
        for (source, selected) in self.decisions.transfer_entries() {
            if source.module_id != self.id {
                return Err(ProviderError::internal(format!(
                    "control transfer {source:?} belongs to another module"
                )));
            }
            let source = source
                .local_id
                .try_into_typed::<dir::Expression>()
                .map_err(ProviderError::internal)?;
            if selected == target
                && matches!(view.get(source), dir::Expression::Continue { .. })
                && !self.flows.is_unreachable(source.into_any())
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one loop has a break carrying a value.
    pub(crate) fn has_valued_break(
        &self,
        iteration: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // require a value-carrying loop target
        if !matches!(self.view().get(iteration), dir::Expression::Loop { .. }) {
            let iteration = iteration.into_global_any(self.id);

            return Err(ProviderError::internal(format!(
                "valued break query target {iteration:?} is not a loop expression"
            )));
        }

        let target = iteration.into_global(self.id);
        let view = self.view();

        // inspect checked breaks selecting this loop
        for (source, selected) in self.decisions.transfer_entries() {
            if source.module_id != self.id {
                return Err(ProviderError::internal(format!(
                    "control transfer {source:?} belongs to another module"
                )));
            }
            let source = source
                .local_id
                .try_into_typed::<dir::Expression>()
                .map_err(ProviderError::internal)?;
            if selected == target
                && matches!(
                    view.get(source),
                    dir::Expression::Break { value: Some(_), .. }
                )
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one subtree uses control from its enclosing context.
    pub(crate) fn uses_enclosing_control(
        &self,
        subtree: dir::LocalNodeIdAny,
    ) -> Result<bool, ProviderError> {
        let view = self.view();
        let callable = self.enclosing_callable_body(subtree);

        // inspect control expressions owned by the same callable
        for (node, control) in view.iter_nodes::<dir::Expression>() {
            if !view.is_inside(node.into_any(), subtree)
                || self.enclosing_callable_body(node.into_any()) != callable
            {
                continue;
            }
            if matches!(
                control,
                dir::Expression::Await { .. }
                    | dir::Expression::AwaitMaybe { .. }
                    | dir::Expression::AwaitMust { .. }
                    | dir::Expression::Yield { .. }
                    | dir::Expression::Return { .. }
                    | dir::Expression::Maybe { .. }
            ) {
                return Ok(true);
            }

            // recognize transfers whose selected target remains outside the subtree
            if matches!(
                control,
                dir::Expression::Break { .. } | dir::Expression::Continue { .. }
            ) && !view.is_inside(self.transfer_target(node)?.into_any(), subtree)
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one expression occupies a terminal path within one block.
    pub(crate) fn is_terminal_in(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> bool {
        let view = self.view();
        let mut current = expression.into_any();

        // climb constructs whose completion continues at their parent's end
        while let Some(parent) = view.get_parent_any(current) {
            match parent.ty {
                // require the current expression to finish its block
                dir::NodeType::Block => {
                    let block = dir::LocalNodeId::<dir::Block>::new(parent.id);
                    let Ok(expression) = current.try_into_typed::<dir::Expression>() else {
                        return false;
                    };
                    if view.get(block).last_expression() != Some(expression) {
                        return false;
                    }
                    if block == body {
                        return true;
                    }

                    current = parent;
                }
                // cross branch bodies and expression block wrappers
                dir::NodeType::Expression => {
                    let expression = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                    let node = view.get(expression);
                    let is_terminal_child = matches!(node, dir::Expression::Block(_))
                        || matches!(
                            node,
                            dir::Expression::If {
                                then_expression,
                                else_expression,
                                ..
                            } if current == then_expression.into_any()
                                || else_expression.is_some_and(|branch| current == branch.into_any())
                        )
                        || matches!(
                            node,
                            dir::Expression::Match { .. }
                                if current.ty == dir::NodeType::MatchArm
                        )
                        || matches!(
                            node,
                            dir::Expression::Try {
                                body,
                                catch,
                                ..
                            } if current == body.into_any()
                                || catch.is_some_and(|branch| current == branch.into_any())
                        )
                        || matches!(
                            node,
                            dir::Expression::Switch { cases, .. }
                                if cases.last().is_some_and(|case| current == case.into_any())
                        );
                    if !is_terminal_child {
                        return false;
                    }

                    current = parent;
                }
                // cross the body of one switch case
                dir::NodeType::SwitchCase => {
                    let case = dir::LocalNodeId::<dir::SwitchCase>::new(parent.id);
                    if current != view.get(case).body.into_any() {
                        return false;
                    }

                    current = parent;
                }
                // cross either body form of one match arm
                dir::NodeType::MatchArm => {
                    let arm = dir::LocalNodeId::<dir::MatchArm>::new(parent.id);
                    let is_body = match view.get(arm) {
                        dir::MatchArm::Expression { body, .. } => current == body.into_any(),
                        dir::MatchArm::Block { body, .. } => current == body.into_any(),
                    };
                    if !is_body {
                        return false;
                    }

                    current = parent;
                }
                // cross the body of one catch branch
                dir::NodeType::Catch => {
                    let catch = dir::LocalNodeId::<dir::Catch>::new(parent.id);
                    if current != view.get(catch).body.into_any() {
                        return false;
                    }

                    current = parent;
                }
                // stop at parents that alter completion behavior
                _ => return false,
            }
        }

        false
    }
}
