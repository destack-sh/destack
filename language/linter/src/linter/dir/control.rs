use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::{DirModule, IntegerStep};

/// One conditional branch in an authored if chain.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IfBranch<'a> {
    /// The branch condition.
    pub(crate) condition: &'a dir::Condition,
    /// The branch body expression.
    pub(crate) body: dir::LocalNodeId<dir::Expression>,
}

/// One authored if-else chain.
#[derive(Debug)]
pub(crate) struct IfChain<'a> {
    /// The conditional branches in source order.
    pub(crate) branches: Vec<IfBranch<'a>>,
    /// The final else expression.
    pub(crate) else_body: Option<dir::LocalNodeId<dir::Expression>>,
}

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

/// One increasing counted iteration.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CountedIteration {
    /// The counter binding, when named.
    pub(crate) binding: Option<dir::GlobalSymbolId>,
    /// The first yielded integer.
    pub(crate) start: i64,
    /// The upper bound expression.
    pub(crate) end: dir::LocalNodeId<dir::Expression>,
    /// Whether the upper bound is excluded or included.
    pub(crate) end_kind: dir::RangeEnd,
    /// Whether the upper bound is evaluated before every iteration.
    pub(crate) is_end_rechecked: bool,
    /// The iteration body.
    pub(crate) body: dir::LocalNodeId<dir::Block>,
}

impl CountedIteration {
    /// Return the iteration count for one constant upper bound.
    pub(crate) fn count(self, end: i64) -> i128 {
        let distance = i128::from(end) - i128::from(self.start);

        match self.end_kind {
            dir::RangeEnd::Open => distance.max(0),
            dir::RangeEnd::Inclusive => (distance + 1).max(0),
        }
    }
}

impl DirModule<'_> {
    /// Return whether one if expression continues a direct else-if chain.
    pub(crate) fn is_else_if(&self, expression: dir::LocalNodeId<dir::Expression>) -> bool {
        // require a direct expression parent
        let view = self.view();
        let Some(parent) = view.get_parent_for(expression) else {
            return false;
        };
        let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
            return false;
        };

        matches!(
            view.get(parent),
            dir::Expression::If {
                form: dir::IfForm::If,
                else_expression: Some(else_expression),
                ..
            } if *else_expression == expression
        )
    }

    /// Return the conditional branches and final else body of one if chain.
    pub(crate) fn if_chain(
        &self,
        mut expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<IfChain<'_>> {
        let view = self.view();
        let mut branches = Vec::new();

        // follow direct else-if expressions in source order
        let else_body = loop {
            let dir::Expression::If {
                form: dir::IfForm::If,
                condition,
                then_expression,
                else_expression,
                ..
            } = view.get(expression)
            else {
                return None;
            };
            branches.push(IfBranch {
                condition,
                body: *then_expression,
            });

            // continue at an else-if or retain the final else body
            let Some(next) = else_expression else {
                break None;
            };
            if matches!(
                view.get(*next),
                dir::Expression::If {
                    form: dir::IfForm::If,
                    ..
                }
            ) {
                expression = *next;
            } else {
                break Some(*next);
            }
        };

        Some(IfChain {
            branches,
            else_body,
        })
    }

    /// Return the coverage proved for one match expression.
    pub(crate) fn match_coverage(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::CoverageDecision> {
        match self.decisions.decision(expression.into_global_any(self.id)) {
            Some(dir::Decision::Coverage(coverage)) => Some(coverage),
            _ => None,
        }
    }

    /// Return the try expression that catches propagation from one node.
    pub(crate) fn catching_try(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let mut current = node;

        // climb through the current callable to the nearest catching try body
        while let Some(parent) = view.get_parent_any(current) {
            if self.callable_body(parent).is_some_and(|body| {
                current
                    .try_into_typed::<dir::Expression>()
                    .is_ok_and(|current| current == body)
            }) {
                return None;
            }

            // select try bodies with an active catch target
            if let Ok(expression) = parent.try_into_typed::<dir::Expression>()
                && let dir::Expression::Try {
                    body,
                    catch: Some(_),
                    ..
                } = view.get(expression)
                && current == body.into_any()
            {
                return Some(expression);
            }

            current = parent;
        }

        None
    }

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

    /// Return the target of one unguarded arm containing only a valueless break.
    pub(crate) fn break_arm_target(
        &self,
        arm: dir::LocalNodeId<dir::MatchArm>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        let Some((_, body)) = self.match_arm_expression(arm) else {
            return Ok(None);
        };

        self.plain_break_target(body)
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

    /// Select one synchronous increasing counted iteration.
    pub(crate) fn counted_iteration(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<CountedIteration>, ProviderError> {
        match self.view().get(expression) {
            // select classic counter loops
            dir::Expression::For {
                initialization: Some(initialization),
                condition: Some(condition),
                increment: Some(increment),
                body,
                ..
            } => self.counted_counter(*initialization, *condition, *increment, *body),

            // select authored ranges
            dir::Expression::ForEach {
                asynchrony: dir::Asynchrony::Sync,
                binding,
                iterator,
                body,
                ..
            } => self.counted_range(binding, *iterator, *body),
            _ => Ok(None),
        }
    }

    /// Select one classic increasing counted loop.
    fn counted_counter(
        &self,
        initialization: dir::LocalNodeId<dir::Expression>,
        condition: dir::LocalNodeId<dir::Expression>,
        increment: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> Result<Option<CountedIteration>, ProviderError> {
        // require one mutable counter with a constant initial value
        let Some((dir::Mutability::Mutable, declarator)) = self.binding_declarator(initialization)
        else {
            return Ok(None);
        };
        let Some(initializer) = declarator.value else {
            return Ok(None);
        };
        let Some(start) = self.integral_constant(initializer)? else {
            return Ok(None);
        };
        let binding = self.declaration_symbol(declarator.pattern)?;

        // require one increasing unit step over the counter
        let Some(IntegerStep::Increment(target)) = self.integer_update(increment)? else {
            return Ok(None);
        };
        if self.selected_symbol(target)? != Some(binding) {
            return Ok(None);
        }

        // normalize either operand order of the upper bound
        let Some((operator, [left, right])) = self.builtin_binary(condition)? else {
            return Ok(None);
        };
        let (end, end_kind) = match operator {
            dir::BinaryOperator::LessThan
                if self.selected_symbol(left.source.local_id)? == Some(binding) =>
            {
                (right.source.local_id, dir::RangeEnd::Open)
            }
            dir::BinaryOperator::LessThanOrEqual
                if self.selected_symbol(left.source.local_id)? == Some(binding) =>
            {
                (right.source.local_id, dir::RangeEnd::Inclusive)
            }
            dir::BinaryOperator::GreaterThan
                if self.selected_symbol(right.source.local_id)? == Some(binding) =>
            {
                (left.source.local_id, dir::RangeEnd::Open)
            }
            dir::BinaryOperator::GreaterThanOrEqual
                if self.selected_symbol(right.source.local_id)? == Some(binding) =>
            {
                (left.source.local_id, dir::RangeEnd::Inclusive)
            }
            _ => return Ok(None),
        };

        Ok(Some(CountedIteration {
            binding: Some(binding),
            start,
            end,
            end_kind,
            is_end_rechecked: true,
            body,
        }))
    }

    /// Select one authored finite integer range.
    fn counted_range(
        &self,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> Result<Option<CountedIteration>, ProviderError> {
        // require a wildcard or direct declared binding
        let dir::ForEachBinding::Pattern {
            pattern,
            keyword: Some(_),
        } = binding
        else {
            return Ok(None);
        };
        let binding = match self.view().get(*pattern) {
            dir::Pattern::Wildcard => None,
            dir::Pattern::Binding { pattern: None, .. } => Some(self.declaration_symbol(*pattern)?),
            _ => return Ok(None),
        };

        // require finite bounds with a constant integer start
        let dir::Expression::RangeExpression {
            start: Some(start),
            end: Some(end),
            end_kind,
        } = self.view().get(iterator)
        else {
            return Ok(None);
        };
        let Some(start) = self.integral_constant(*start)? else {
            return Ok(None);
        };

        Ok(Some(CountedIteration {
            binding,
            start,
            end: *end,
            end_kind: *end_kind,
            is_end_rechecked: false,
            body,
        }))
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

    /// Return the nearest iteration containing one node in the same callable.
    pub(crate) fn enclosing_iteration(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let mut parent = view.get_parent_any(node);

        // climb to the nearest iteration or callable boundary
        while let Some(node) = parent {
            // stop before crossing nested callable ownership
            if self.callable_body(node).is_some() {
                return None;
            }

            // select the nearest iteration expression
            if let Ok(expression) = node.try_into_typed::<dir::Expression>()
                && self.iteration_body(expression).is_some()
            {
                return Some(expression);
            }

            parent = view.get_parent_any(node);
        }

        None
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

    /// Return the control target selected by one break or continue expression.
    pub(crate) fn transfer_target(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::LocalNodeId<dir::Expression>, ProviderError> {
        // read the target selected for this transfer
        let global = expression.into_global_any(self.id);
        let target = self.decisions.transfer_decision(global).ok_or_else(|| {
            ProviderError::internal(format!(
                "control transfer {global:?} has no target decision"
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

    /// Return the target of one sole valueless break expression.
    pub(crate) fn plain_break_target(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        let Some(expression) = self.sole_expression(expression) else {
            return Ok(None);
        };
        if !matches!(
            self.view().get(expression),
            dir::Expression::Break { value: None, .. }
        ) {
            return Ok(None);
        }

        self.transfer_target(expression).map(Some)
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

        // inspect transfers selecting this iteration
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

        // inspect breaks selecting this loop
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
