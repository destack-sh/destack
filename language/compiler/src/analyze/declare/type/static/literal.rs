use crate::analyze::common::TypeContext;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Argument, BinaryOperator, Expression, LocalNodeId, LocalTypeId, PrimitiveType, ScalarLiteral,
    StaticArgument, StaticExpression, Type, TypeLiteral, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn evaluate_integer_static_literal(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<i64>> {
        // prefer existing type commitments before re-evaluating the expression tree
        let expression_global = expression_id.into_global_any(ctx.module.id);
        if let Some(type_id) = ctx
            .types
            .get_inferred_type_id(expression_global)
            .or_else(|| ctx.types.get_declared_type_id(expression_global))
            && let Some(value) = self.integer_literal_value_for_type_id(type_id, ctx.types)
        {
            return Ok(Some(value));
        }

        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, ctx.tree);
        let value =
            self.evaluate_static_expression_value(&mut ctx.reborrow(), expression_id, None)?;
        let literal = match value {
            Some(StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            }) => Some(value),
            Some(StaticExpression::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            }) => Some(value),
            Some(StaticExpression::Type { ty }) => match ctx.types.get_type(ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => Some(*value),
                _ => None,
            },
            _ => None,
        };
        Ok(literal)
    }

    /// Convert one substituted static parameter type into a static expression.
    pub(crate) fn static_expression_from_substitution_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> StaticExpression {
        let type_id = types.unwrap_value_type_id(type_id);
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Type::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            _ => StaticExpression::Type { ty: type_id },
        }
    }

    /// Return true when one binary operator can be deferred for symbolic static evaluation.
    pub(crate) fn binary_operator_supports_symbolic_static_evaluation(
        &self,
        operator: BinaryOperator,
    ) -> bool {
        matches!(
            operator,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Remainder
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::UnsignedShiftRight
                | BinaryOperator::ElementwiseAnd
                | BinaryOperator::ElementwiseOr
                | BinaryOperator::ElementwiseXor
        )
    }

    /// Return true when one static expression can participate in symbolic numeric evaluation.
    pub(crate) fn static_expression_may_be_numeric(
        &self,
        value: &StaticExpression,
        types: &TypeTable,
    ) -> bool {
        if self.static_expression_integer_value(value, types).is_some() {
            return true;
        }

        match value {
            StaticExpression::Type { ty } => {
                let mut current = *ty;
                for _ in 0..8 {
                    match types.get_type(current) {
                        Type::Value { value } => current = *value,
                        Type::Reference { .. } | Type::Unevaluated(_) => return true,
                        Type::TypeLiteral {
                            value:
                                TypeLiteral::Primitive(PrimitiveType::Number | PrimitiveType::Int(_)),
                        } => return true,
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        } => return true,
                        _ => return false,
                    }
                }
                false
            }
            StaticExpression::Unevaluated { .. } => true,
            _ => false,
        }
    }

    /// Extract one integer literal value from a static expression when possible.
    pub(crate) fn static_expression_integer_value(
        &self,
        value: &StaticExpression,
        types: &TypeTable,
    ) -> Option<i64> {
        match value {
            StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } => Some(*value),
            StaticExpression::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            } => Some(*value),
            StaticExpression::Type { ty } => match types.get_type(*ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => Some(*value),
                Type::Value { value } => match types.get_type(*value) {
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                    } => Some(*value),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn evaluate_static_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // skip when there are no static arguments
        let Some(static_arguments) = static_arguments else {
            return Ok(None);
        };

        let mut evaluated_arguments = Vec::with_capacity(static_arguments.len());

        // defer static argument evaluation until parameter kinds are known
        for argument_id in static_arguments {
            evaluated_arguments.push(StaticArgument::Unevaluated {
                node: argument_id.into_global_any(ctx.module.id),
            });
        }

        Ok(Some(evaluated_arguments))
    }
}
