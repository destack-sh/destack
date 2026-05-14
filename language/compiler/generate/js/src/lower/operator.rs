use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a DIR unary expression to a JS unary expression.
    pub fn lower_unary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Expression>> {
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<js::Expression>(right_id.into_global_any(self.module.id), self)?;

        // lower a trivial unary expression to a JS unary expression
        let mut unary = |operator: js::UnaryOperator| -> js::LocalNodeId<js::Expression> {
            let expression = js::Expression::Unary {
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            dir::UnaryOperator::PostIncrement => unary(js::UnaryOperator::PostIncrement),
            dir::UnaryOperator::PostDecrement => unary(js::UnaryOperator::PostDecrement),
            dir::UnaryOperator::PreIncrement => unary(js::UnaryOperator::PreIncrement),
            dir::UnaryOperator::PreDecrement => unary(js::UnaryOperator::PreDecrement),
            dir::UnaryOperator::Not => unary(js::UnaryOperator::Not),
            dir::UnaryOperator::Plus => unary(js::UnaryOperator::Plus),
            dir::UnaryOperator::Negate => unary(js::UnaryOperator::Negate),
            dir::UnaryOperator::ElementwiseNot => unary(js::UnaryOperator::ElementwiseNot),
            dir::UnaryOperator::Typeof => unary(js::UnaryOperator::Typeof),
            dir::UnaryOperator::Void => unary(js::UnaryOperator::Void),
            dir::UnaryOperator::Dereference => right_id,
            dir::UnaryOperator::Spread => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some("spread unary expressions are not lowered to JS".to_string()),
                });
            }
        };

        Ok(expression_id)
    }

    /// Lower a DIR binary expression to a JS binary expression.
    pub fn lower_binary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Expression>> {
        let left_id = self
            .lower_expression(left_id)
            .expect_node::<js::Expression>(left_id.into_global_any(self.module.id), self)?;
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<js::Expression>(right_id.into_global_any(self.module.id), self)?;

        let mut binary = |operator: js::BinaryOperator| -> js::LocalNodeId<js::Expression> {
            let expression = js::Expression::Binary {
                left: left_id,
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            dir::BinaryOperator::Multiply => binary(js::BinaryOperator::Multiply),
            dir::BinaryOperator::Exponent => binary(js::BinaryOperator::Exponent),
            dir::BinaryOperator::Divide => binary(js::BinaryOperator::Divide),
            dir::BinaryOperator::Remainder => binary(js::BinaryOperator::Remainder),
            dir::BinaryOperator::Add => binary(js::BinaryOperator::Add),
            dir::BinaryOperator::Subtract => binary(js::BinaryOperator::Subtract),
            dir::BinaryOperator::ShiftLeft => binary(js::BinaryOperator::ShiftLeft),
            dir::BinaryOperator::ShiftRight => binary(js::BinaryOperator::ShiftRight),
            dir::BinaryOperator::UnsignedShiftRight => {
                binary(js::BinaryOperator::UnsignedShiftRight)
            }
            dir::BinaryOperator::ElementwiseAnd => binary(js::BinaryOperator::ElementwiseAnd),
            dir::BinaryOperator::ElementwiseXor => binary(js::BinaryOperator::ElementwiseXor),
            dir::BinaryOperator::ElementwiseOr => binary(js::BinaryOperator::ElementwiseOr),
            dir::BinaryOperator::Equal => binary(js::BinaryOperator::Equal),
            dir::BinaryOperator::NotEqual => binary(js::BinaryOperator::NotEqual),
            dir::BinaryOperator::EqualStrict => binary(js::BinaryOperator::EqualStrict),
            dir::BinaryOperator::NotEqualStrict => binary(js::BinaryOperator::NotEqualStrict),
            dir::BinaryOperator::LessThan => binary(js::BinaryOperator::LessThan),
            dir::BinaryOperator::LessThanOrEqual => binary(js::BinaryOperator::LessThanOrEqual),
            dir::BinaryOperator::GreaterThan => binary(js::BinaryOperator::GreaterThan),
            dir::BinaryOperator::GreaterThanOrEqual => {
                binary(js::BinaryOperator::GreaterThanOrEqual)
            }
            dir::BinaryOperator::And => binary(js::BinaryOperator::And),
            dir::BinaryOperator::Or => binary(js::BinaryOperator::Or),
            dir::BinaryOperator::Coalesce => binary(js::BinaryOperator::Coalesce),
            dir::BinaryOperator::In => binary(js::BinaryOperator::In),
        };

        Ok(expression_id)
    }

    /// Lower a DIR assign binary expression to a JS assign binary expression.
    pub fn lower_assign_binary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::AssignOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Expression>> {
        let left_id = self
            .lower_expression(left_id)
            .expect_node::<js::Expression>(left_id.into_global_any(self.module.id), self)?;
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<js::Expression>(right_id.into_global_any(self.module.id), self)?;

        // lower a trivial assign binary expression to a JS assign binary expression
        let mut assign_binary = |operator: js::AssignOperator| -> js::LocalNodeId<js::Expression> {
            let expression = js::Expression::AssignBinary {
                left: left_id,
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            dir::AssignOperator::Assign => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some("plain assignment is not a compound assignment".to_string()),
                });
            }
            dir::AssignOperator::MultiplyAssign => {
                assign_binary(js::AssignOperator::MultiplyAssign)
            }
            dir::AssignOperator::DivideAssign => assign_binary(js::AssignOperator::DivideAssign),
            dir::AssignOperator::RemainderAssign => {
                assign_binary(js::AssignOperator::RemainderAssign)
            }
            dir::AssignOperator::ExponentAssign => {
                assign_binary(js::AssignOperator::ExponentAssign)
            }
            dir::AssignOperator::AddAssign => assign_binary(js::AssignOperator::AddAssign),
            dir::AssignOperator::SubtractAssign => {
                assign_binary(js::AssignOperator::SubtractAssign)
            }
            dir::AssignOperator::ShiftLeftAssign => {
                assign_binary(js::AssignOperator::ShiftLeftAssign)
            }
            dir::AssignOperator::ShiftRightAssign => {
                assign_binary(js::AssignOperator::ShiftRightAssign)
            }
            dir::AssignOperator::UnsignedShiftRightAssign => {
                assign_binary(js::AssignOperator::UnsignedShiftRightAssign)
            }
            dir::AssignOperator::ElementwiseAndAssign => {
                assign_binary(js::AssignOperator::ElementwiseAndAssign)
            }
            dir::AssignOperator::ElementwiseXorAssign => {
                assign_binary(js::AssignOperator::ElementwiseXorAssign)
            }
            dir::AssignOperator::ElementwiseOrAssign => {
                assign_binary(js::AssignOperator::ElementwiseOrAssign)
            }
            dir::AssignOperator::AndAssign => assign_binary(js::AssignOperator::AndAssign),
            dir::AssignOperator::OrAssign => assign_binary(js::AssignOperator::OrAssign),
            dir::AssignOperator::CoalesceAssign => {
                assign_binary(js::AssignOperator::CoalesceAssign)
            }
        };

        Ok(expression_id)
    }
}
