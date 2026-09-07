use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Emit a DIR unary expression to a JavaScript unary expression.
    pub(crate) fn emit_unary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let expression = match operator {
            // emit an update expression
            dir::UnaryOperator::PostIncrement
            | dir::UnaryOperator::PostDecrement
            | dir::UnaryOperator::PreIncrement
            | dir::UnaryOperator::PreDecrement => {
                let place = self.emit_place(right_id)?;
                let update_operator = match operator {
                    dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                        js::UpdateOperator::Increment
                    }
                    dir::UnaryOperator::PostDecrement | dir::UnaryOperator::PreDecrement => {
                        js::UpdateOperator::Decrement
                    }
                    _ => unreachable!("update operator group changed during JavaScript emission"),
                };
                let position = match operator {
                    dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PostDecrement => {
                        js::UpdatePosition::Postfix
                    }
                    dir::UnaryOperator::PreIncrement | dir::UnaryOperator::PreDecrement => {
                        js::UpdatePosition::Prefix
                    }
                    _ => unreachable!("update operator group changed during JavaScript emission"),
                };

                js::Expression::Update {
                    place,
                    operator: update_operator,
                    position,
                }
            }
            // emit a unary expression
            dir::UnaryOperator::Not
            | dir::UnaryOperator::Plus
            | dir::UnaryOperator::Negate
            | dir::UnaryOperator::ElementwiseNot => {
                let right = self.emit_expression(right_id)?;
                let operator = match operator {
                    dir::UnaryOperator::Not => js::UnaryOperator::Not,
                    dir::UnaryOperator::Plus => js::UnaryOperator::Plus,
                    dir::UnaryOperator::Negate => js::UnaryOperator::Negate,
                    dir::UnaryOperator::ElementwiseNot => js::UnaryOperator::ElementwiseNot,
                    _ => unreachable!("unary operator group changed during JavaScript emission"),
                };

                js::Expression::Unary { operator, right }
            }
            // erase DIR dereferences
            dir::UnaryOperator::Dereference => {
                return self.emit_expression(right_id);
            }
            // reject spread outside its owning construct
            dir::UnaryOperator::Spread => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("spread unary expressions cannot reach JavaScript emission".to_string()),
                ));
            }
        };

        Ok(self.insert_from_source(expression, expression_id))
    }

    /// Emit a DIR binary expression to a JavaScript binary expression.
    pub(crate) fn emit_binary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        // emit both operands
        let left = self.emit_expression(left_id)?;
        let right = self.emit_expression(right_id)?;

        // map the binary operator
        let operator = match operator {
            dir::BinaryOperator::Multiply => js::BinaryOperator::Multiply,
            dir::BinaryOperator::Exponent => js::BinaryOperator::Exponent,
            dir::BinaryOperator::Divide => js::BinaryOperator::Divide,
            dir::BinaryOperator::Remainder => js::BinaryOperator::Remainder,
            dir::BinaryOperator::Add => js::BinaryOperator::Add,
            dir::BinaryOperator::Subtract => js::BinaryOperator::Subtract,
            dir::BinaryOperator::ShiftLeft => js::BinaryOperator::ShiftLeft,
            dir::BinaryOperator::ShiftRight => js::BinaryOperator::ShiftRight,
            dir::BinaryOperator::UnsignedShiftRight => js::BinaryOperator::UnsignedShiftRight,
            dir::BinaryOperator::ElementwiseAnd => js::BinaryOperator::ElementwiseAnd,
            dir::BinaryOperator::ElementwiseXor => js::BinaryOperator::ElementwiseXor,
            dir::BinaryOperator::ElementwiseOr => js::BinaryOperator::ElementwiseOr,
            dir::BinaryOperator::Equal => js::BinaryOperator::Equal,
            dir::BinaryOperator::NotEqual => js::BinaryOperator::NotEqual,
            dir::BinaryOperator::EqualStrict => js::BinaryOperator::EqualStrict,
            dir::BinaryOperator::NotEqualStrict => js::BinaryOperator::NotEqualStrict,
            dir::BinaryOperator::LessThan => js::BinaryOperator::LessThan,
            dir::BinaryOperator::LessThanOrEqual => js::BinaryOperator::LessThanOrEqual,
            dir::BinaryOperator::GreaterThan => js::BinaryOperator::GreaterThan,
            dir::BinaryOperator::GreaterThanOrEqual => js::BinaryOperator::GreaterThanOrEqual,
            dir::BinaryOperator::And => js::BinaryOperator::And,
            dir::BinaryOperator::Or => js::BinaryOperator::Or,
            dir::BinaryOperator::Coalesce => js::BinaryOperator::Coalesce,
            dir::BinaryOperator::In => js::BinaryOperator::In,
        };

        // build the binary expression
        let expression = js::Expression::Binary {
            left,
            operator,
            right,
        };

        Ok(self.insert_from_source(expression, expression_id))
    }

    /// Emit a DIR compound assignment to a JavaScript compound assignment.
    pub(crate) fn emit_assign_binary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::AssignOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        // map the compound operator
        let operator = match operator {
            dir::AssignOperator::Assign => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("plain assignment is not a compound assignment".to_string()),
                ));
            }
            dir::AssignOperator::MultiplyAssign => js::AssignOperator::MultiplyAssign,
            dir::AssignOperator::DivideAssign => js::AssignOperator::DivideAssign,
            dir::AssignOperator::RemainderAssign => js::AssignOperator::RemainderAssign,
            dir::AssignOperator::ExponentAssign => js::AssignOperator::ExponentAssign,
            dir::AssignOperator::AddAssign => js::AssignOperator::AddAssign,
            dir::AssignOperator::SubtractAssign => js::AssignOperator::SubtractAssign,
            dir::AssignOperator::ShiftLeftAssign => js::AssignOperator::ShiftLeftAssign,
            dir::AssignOperator::ShiftRightAssign => js::AssignOperator::ShiftRightAssign,
            dir::AssignOperator::UnsignedShiftRightAssign => {
                js::AssignOperator::UnsignedShiftRightAssign
            }
            dir::AssignOperator::ElementwiseAndAssign => js::AssignOperator::ElementwiseAndAssign,
            dir::AssignOperator::ElementwiseXorAssign => js::AssignOperator::ElementwiseXorAssign,
            dir::AssignOperator::ElementwiseOrAssign => js::AssignOperator::ElementwiseOrAssign,
            dir::AssignOperator::AndAssign => js::AssignOperator::AndAssign,
            dir::AssignOperator::OrAssign => js::AssignOperator::OrAssign,
            dir::AssignOperator::CoalesceAssign => js::AssignOperator::CoalesceAssign,
        };

        // emit both operands
        let left = self.emit_place(left_id)?;
        let right = self.emit_expression(right_id)?;

        // build the compound assignment
        let expression = js::Expression::AssignBinary {
            left,
            operator,
            right,
        };

        Ok(self.insert_from_source(expression, expression_id))
    }
}
