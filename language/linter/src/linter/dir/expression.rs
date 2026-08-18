use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the sole direct binding declarator in one declaration expression.
    pub(crate) fn binding_declarator(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<(dir::Mutability, &dir::Declarator)> {
        // select one declaration with one declarator
        let view = self.view();
        let dir::Expression::Let {
            mutability,
            declarators,
            ..
        } = view.get(expression)
        else {
            return None;
        };
        let [declarator] = declarators.as_slice() else {
            return None;
        };
        let declarator = view.get(*declarator);

        // require a direct rather than destructured binding
        if !matches!(
            view.get(declarator.pattern),
            dir::Pattern::Binding { pattern: None, .. }
        ) {
            return None;
        }

        Some((*mutability, declarator))
    }

    /// Return the expression that directly produces one expression's value.
    pub(crate) fn value_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self.view().get(expression) {
            dir::Expression::Block(block) => self.view().get(*block).value_expression(),
            _ => Some(expression),
        }
    }

    /// Return every terminal expression that can produce one expression's value.
    pub(super) fn terminal_values(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<dir::LocalNodeId<dir::Expression>>> {
        let mut values = Vec::new();
        if !self.collect_terminal_values(expression, &mut values) {
            return None;
        }

        Some(values)
    }

    /// Append every terminal expression that can produce one expression's value.
    fn collect_terminal_values(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        values: &mut Vec<dir::LocalNodeId<dir::Expression>>,
    ) -> bool {
        let view = self.view();

        // follow block tail values
        if let dir::Expression::Block(block) = view.get(expression) {
            let Some(value) = view.get(*block).value_expression() else {
                return true;
            };

            return self.collect_terminal_values(value, values);
        }

        // follow both conditional branches
        if let dir::Expression::If {
            then_expression,
            else_expression,
            ..
        } = view.get(expression)
        {
            let Some(else_expression) = else_expression else {
                return false;
            };

            return self.collect_terminal_values(*then_expression, values)
                && self.collect_terminal_values(*else_expression, values);
        }

        // follow every match arm
        if let dir::Expression::Match { arms, .. } = view.get(expression) {
            if arms.is_empty() {
                return false;
            }
            for arm in arms {
                let value = match view.get(*arm) {
                    dir::MatchArm::Expression { body, .. } => Some(*body),
                    dir::MatchArm::Block { body, .. } => view.get(*body).value_expression(),
                };
                let Some(value) = value else {
                    return false;
                };
                if !self.collect_terminal_values(value, values) {
                    return false;
                }
            }

            return true;
        }

        // retain one indivisible value from a normally completing path
        if !self.flows.is_diverging(expression.into_any()) {
            values.push(expression);
        }

        true
    }

    /// Return the sole expression performed directly or within one block.
    pub(crate) fn sole_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self.view().get(expression) {
            dir::Expression::Block(block) => self.view().get(*block).only_expression(),
            _ => Some(expression),
        }
    }

    /// Return the value expression when its block performs no preceding work.
    pub(crate) fn sole_value_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let dir::Expression::Block(block) = view.get(expression) else {
            return Some(expression);
        };
        let block = view.get(*block);
        if !block.leading_expressions.is_empty() {
            return None;
        }

        block.value_expression()
    }

    /// Return the nearest expression that contains one expression in value position.
    pub(crate) fn enclosing_value_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let mut current = expression.into_any();

        // cross structural nodes until reaching an expression or body boundary
        while let Some(parent) = view.get_parent_any(current) {
            match parent.ty {
                // cross an expression block only through its value tail
                dir::NodeType::Block => {
                    let block = dir::LocalNodeId::<dir::Block>::new(parent.id);
                    let Ok(current_expression) = current.try_into_typed::<dir::Expression>() else {
                        return None;
                    };
                    if view.get(block).value_expression() != Some(current_expression) {
                        return None;
                    }

                    current = parent;
                }
                // stop at branch bodies, otherwise return the value parent
                dir::NodeType::Expression => {
                    let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                    let is_body = match view.get(parent) {
                        dir::Expression::If {
                            form: dir::IfForm::If,
                            then_expression,
                            else_expression,
                            ..
                        } => {
                            current == then_expression.into_any()
                                || else_expression.is_some_and(|body| current == body.into_any())
                        }
                        dir::Expression::LetElse { else_branch, .. } => {
                            current == else_branch.into_any()
                        }
                        dir::Expression::Try { body, finally, .. } => {
                            current == body.into_any()
                                || finally.is_some_and(|body| current == body.into_any())
                        }
                        _ => false,
                    };
                    if is_body {
                        return None;
                    }

                    return Some(parent);
                }
                // stop at bindings and control branches
                dir::NodeType::Declarator
                | dir::NodeType::MatchArm
                | dir::NodeType::Catch
                | dir::NodeType::SwitchCase => return None,
                _ => {
                    // stop at callable ownership
                    if current
                        .try_into_typed::<dir::Expression>()
                        .ok()
                        .is_some_and(|body| self.callable_body(parent) == Some(body))
                    {
                        return None;
                    }

                    // cross transparent authored nodes
                    current = parent;
                }
            }
        }

        None
    }

    /// Return whether one checked expression can be evaluated without observable effects.
    pub fn is_repeatable_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        if !self.is_same_computation(expression, expression)? {
            return Ok(false);
        }

        self.can_trap(expression).map(|can_trap| !can_trap)
    }

    /// Return whether evaluating one checked expression can trap.
    fn can_trap(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        let node = self.view().get(expression);

        // accept scalar values
        if node.as_scalar().is_some() {
            return Ok(false);
        }

        // inspect stable storage without hiding receiver evaluation
        let can_trap = match node {
            dir::Expression::Identifier { .. } | dir::Expression::This | dir::Expression::Super
                if self.access_resolution(expression).is_some() =>
            {
                false
            }
            dir::Expression::Member { left, .. }
                if self.access_resolution(expression).is_some() =>
            {
                self.can_trap(*left)?
            }
            dir::Expression::Chain { expression } => self.can_trap(*expression)?,
            dir::Expression::Index { .. } => true,

            // inspect the repeatable builtin operations
            dir::Expression::Unary { .. } => {
                let Some((operator, operand)) = self.builtin_unary(expression)? else {
                    return Ok(true);
                };
                let operation_can_trap = operator == dir::UnaryOperator::Negate
                    && operand.is_integral()
                    && self.integral_constant(expression)?.is_none();

                operation_can_trap || self.can_trap(operand.source.local_id)?
            }
            dir::Expression::Binary { .. } => {
                let Some((operator, [left, right])) = self.builtin_binary(expression)? else {
                    return Ok(true);
                };
                let operation_can_trap = left.is_integral()
                    && self.integral_constant(expression)?.is_none()
                    && matches!(
                        operator,
                        dir::BinaryOperator::Exponent
                            | dir::BinaryOperator::Multiply
                            | dir::BinaryOperator::Divide
                            | dir::BinaryOperator::Remainder
                            | dir::BinaryOperator::Add
                            | dir::BinaryOperator::Subtract
                            | dir::BinaryOperator::ShiftLeft
                    );

                operation_can_trap
                    || self.can_trap(left.source.local_id)?
                    || self.can_trap(right.source.local_id)?
            }
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => self.can_trap(*expression)?,
            _ => true,
        };

        Ok(can_trap)
    }

    /// Return whether two checked expressions denote the same computation.
    pub fn is_same_computation(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // require identical checked source types
        if self.node_type_id(left.into_any())? != self.node_type_id(right.into_any())? {
            return Ok(false);
        }

        // compare repeatable places through their canonical paths
        let left_access = self.access_resolution(left);
        let right_access = self.access_resolution(right);
        match (left_access, right_access) {
            (Some(left), Some(right)) => return Ok(left == right),
            (Some(_), None) | (None, Some(_)) => return Ok(false),
            (None, None) => {}
        }

        let view = self.view();
        let left_expression = view.get(left);
        let right_expression = view.get(right);

        // compare every scalar expression through its canonical DIR value
        if let (Some(left), Some(right)) =
            (left_expression.as_scalar(), right_expression.as_scalar())
        {
            return Ok(left == right);
        }

        let is_same = match (left_expression, right_expression) {
            // repeatable compiler-defined unary operations
            (dir::Expression::Unary { .. }, dir::Expression::Unary { .. }) => {
                let left_resolution = self.operator_decision(left.into_any())?;
                let right_resolution = self.operator_decision(right.into_any())?;
                let (Some(left_resolution), Some(right_resolution)) =
                    (left_resolution, right_resolution)
                else {
                    return Ok(false);
                };
                let (Some((left_operator, left_operand)), Some((right_operator, right_operand))) = (
                    left_resolution.builtin_unary(),
                    right_resolution.builtin_unary(),
                ) else {
                    return Ok(false);
                };
                if left_operator != right_operator
                    || !matches!(
                        left_operator,
                        dir::UnaryOperator::Not
                            | dir::UnaryOperator::Plus
                            | dir::UnaryOperator::Negate
                            | dir::UnaryOperator::ElementwiseNot
                    )
                {
                    return Ok(false);
                }

                self.is_same_operand(left_operand, right_operand)?
            }

            // repeatable compiler-defined binary operations
            (dir::Expression::Binary { .. }, dir::Expression::Binary { .. }) => {
                let left_resolution = self.operator_decision(left.into_any())?;
                let right_resolution = self.operator_decision(right.into_any())?;
                let (Some(left_resolution), Some(right_resolution)) =
                    (left_resolution, right_resolution)
                else {
                    return Ok(false);
                };
                let (
                    Some((left_operator, [left_first, left_second])),
                    Some((right_operator, [right_first, right_second])),
                ) = (
                    left_resolution.builtin_binary(),
                    right_resolution.builtin_binary(),
                )
                else {
                    return Ok(false);
                };
                if left_operator != right_operator {
                    return Ok(false);
                }

                self.is_same_operand(left_first, right_first)?
                    && self.is_same_operand(left_second, right_second)?
            }

            // compiler-defined casts and static assertions
            (
                dir::Expression::As {
                    expression: left_value,
                    target_type: left_target,
                },
                dir::Expression::As {
                    expression: right_value,
                    target_type: right_target,
                },
            )
            | (
                dir::Expression::Satisfies {
                    expression: left_value,
                    target_type: left_target,
                },
                dir::Expression::Satisfies {
                    expression: right_value,
                    target_type: right_target,
                },
            ) => {
                self.node_type_id(left_target.into_any())?
                    == self.node_type_id(right_target.into_any())?
                    && self.is_same_computation(*left_value, *right_value)?
            }

            // reject computations that can differ between evaluations
            _ => false,
        };

        Ok(is_same)
    }

    /// Return whether two builtin operands produce the same checked value.
    pub fn is_same_operand(
        &self,
        left_operand: &dir::BuiltinOperand,
        right_operand: &dir::BuiltinOperand,
    ) -> Result<bool, ProviderError> {
        if left_operand.ty != right_operand.ty
            || left_operand.scalar_families != right_operand.scalar_families
        {
            return Ok(false);
        }

        let left_coercion = self.coercions.coercion(left_operand.source.into_any());
        let right_coercion = self.coercions.coercion(right_operand.source.into_any());
        if left_coercion != right_coercion {
            return Ok(false);
        }

        let left = left_operand.source.local_id;
        let right = right_operand.source.local_id;

        self.is_same_computation(left, right)
    }
}
