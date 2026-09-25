use crate::parse::expression::operator::ExpressionOperator;
use crate::{Parser, ParserError, ParserResult};
use tspp_dir::{Expression, LocalNodeId, RangeEnd, TypeExpression};
use tspp_source::ByteRange;

impl Parser {
    /// Promote one reference-shaped value expression into type space.
    pub(in crate::parse::expression) fn promote_expression_type(
        &mut self,
        expression: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        match self.tree.get(expression) {
            Expression::Type { value } => Ok(*value),
            _ => self
                .promote_static_type_head(expression)?
                .ok_or_else(|| ParserError::unexpected(self.tree.get_range(expression))),
        }
    }

    /// Insert one value infix expression.
    pub(in crate::parse) fn insert_expression_infix(
        &mut self,
        left: LocalNodeId<Expression>,
        operator: ExpressionOperator,
        operator_range: ByteRange,
        right: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // lower the operation into its DIR form
        let expression = match operator {
            ExpressionOperator::Binary(operator) => Expression::Binary {
                left,
                operator,
                right,
            },
            ExpressionOperator::InstanceOf => Expression::InstanceOf {
                value: left,
                target: right,
            },
            ExpressionOperator::Range(end_kind) => Expression::RangeExpression {
                start: Some(left),
                end: Some(right),
                end_kind,
            },
            ExpressionOperator::Assign(operator) => {
                let left = self.lower_assignment_target(left, operator)?;

                Expression::Assign {
                    left,
                    operator,
                    right,
                }
            }
            ExpressionOperator::Type(_)
            | ExpressionOperator::Is
            | ExpressionOperator::As
            | ExpressionOperator::Satisfies => {
                return Err(ParserError::unexpected(operator_range));
            }
        };

        // cover the complete operation
        let left_range = self.tree.get_source_extent(left).range();
        let right_range = self.tree.get_source_extent(right).range();
        let source_range = ByteRange {
            start: left_range.start,
            end: right_range.end,
        };
        let expression = self.insert_node(expression, source_range);

        // retain the operator as the main source region
        self.tree.set_main_range(expression, operator_range);

        Ok(expression)
    }

    /// Insert one value infix expression with a type operand.
    pub(in crate::parse::expression) fn insert_expression_type_infix(
        &mut self,
        left: LocalNodeId<Expression>,
        operator: ExpressionOperator,
        operator_range: ByteRange,
        target_type: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // lower the operation into its DIR form
        let expression = match operator {
            ExpressionOperator::As => Expression::As {
                expression: left,
                target_type,
            },
            ExpressionOperator::Satisfies => Expression::Satisfies {
                expression: left,
                target_type,
            },
            ExpressionOperator::Is => Expression::Is {
                value: left,
                target_type,
            },
            _ => return Err(ParserError::unexpected(operator_range)),
        };

        // cover the complete operation
        let left_range = self.tree.get_source_extent(left).range();
        let target_range = self.tree.get_source_extent(target_type).range();
        let source_range = ByteRange {
            start: left_range.start,
            end: target_range.end,
        };
        let expression = self.insert_node(expression, source_range);

        // retain trivia between the operator and its type operand
        self.set_node_leading_range(target_type, operator_range.end);

        // treat `as const` as one indivisible operator region
        let main_range = if operator == ExpressionOperator::As
            && matches!(self.tree.get(target_type), TypeExpression::Const)
        {
            ByteRange {
                start: operator_range.start,
                end: target_range.end,
            }
        } else {
            operator_range
        };
        self.tree.set_main_range(expression, main_range);
        self.tree
            .set_head_range(expression, self.expression_head_range(left));

        Ok(expression)
    }

    /// Insert one range expression without a right endpoint.
    pub(in crate::parse) fn insert_open_range_expression(
        &mut self,
        left: LocalNodeId<Expression>,
        operator_range: ByteRange,
    ) -> LocalNodeId<Expression> {
        // cover the left operand through the range operator
        let left_range = self.tree.get_range(left);
        let source_range = ByteRange {
            start: left_range.start,
            end: operator_range.end,
        };
        let expression = self.insert_node(
            Expression::RangeExpression {
                start: Some(left),
                end: None,
                end_kind: RangeEnd::Open,
            },
            source_range,
        );
        self.tree.set_main_range(expression, operator_range);

        expression
    }
}
