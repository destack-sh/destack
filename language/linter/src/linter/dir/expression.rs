use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
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
        self.is_same_computation(expression, expression)
    }

    /// Return whether two checked expressions denote the same repeatable computation.
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
                            | dir::UnaryOperator::Typeof
                            | dir::UnaryOperator::Void
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
