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
