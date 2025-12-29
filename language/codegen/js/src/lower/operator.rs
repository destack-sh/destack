use crate::{
    AssignOperator, BinaryOperator, CodegenJsError, CodegenJsResult, CodegenJsResultExt,
    Expression, LocalNodeId, ModuleLowerer, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a DIR type unary operator to a JS type unary operator.
    pub fn lower_type_unary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::TypeUnaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeId<Expression>> {
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<Expression>(right_id.into_global_any(self.module.id), self)?;

        // lower a trivial unary expression to a JS unary expression
        let mut unary = |operator: TypeUnaryOperator| -> LocalNodeId<Expression> {
            let expression = Expression::TypeUnary {
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            dir::TypeUnaryOperator::Newtype => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
            dir::TypeUnaryOperator::Type => unary(TypeUnaryOperator::Type),
            dir::TypeUnaryOperator::Readonly => unary(TypeUnaryOperator::Readonly),
            dir::TypeUnaryOperator::Not => unary(TypeUnaryOperator::Not),
            dir::TypeUnaryOperator::Must => unary(TypeUnaryOperator::Must),
            dir::TypeUnaryOperator::Typeof => unary(TypeUnaryOperator::Typeof),
            dir::TypeUnaryOperator::Keyof => unary(TypeUnaryOperator::Keyof),
            dir::TypeUnaryOperator::AsConst => unary(TypeUnaryOperator::AsConst),
        };

        Ok(expression_id)
    }

    /// Lower a DIR type binary expression to a JS type binary expression.
    pub fn lower_type_binary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::TypeBinaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeId<Expression>> {
        let left_id = self
            .lower_expression(left_id)
            .expect_node::<Expression>(left_id.into_global_any(self.module.id), self)?;
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<Expression>(right_id.into_global_any(self.module.id), self)?;
        let operator = match operator {
            dir::TypeBinaryOperator::Cast => TypeBinaryOperator::Cast,
            dir::TypeBinaryOperator::In => TypeBinaryOperator::In,
            dir::TypeBinaryOperator::Is => TypeBinaryOperator::Is,
            dir::TypeBinaryOperator::InstanceOf => TypeBinaryOperator::InstanceOf,
            dir::TypeBinaryOperator::Satisfies => TypeBinaryOperator::Satisfies,
            dir::TypeBinaryOperator::Extends => TypeBinaryOperator::Extends,
            dir::TypeBinaryOperator::Implements => TypeBinaryOperator::Implements,
        };
        let expression = Expression::TypeBinary {
            left: left_id,
            operator,
            right: right_id,
        };
        let expression_id = self
            .tree
            .insert_from_source(expression, self.module.id, expression_id);
        Ok(expression_id)
    }

    /// Lower a DIR unary expression to a JS unary expression.
    pub fn lower_unary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeId<Expression>> {
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<Expression>(right_id.into_global_any(self.module.id), self)?;

        // lower a trivial unary expression to a JS unary expression
        let mut unary = |operator: UnaryOperator| -> LocalNodeId<Expression> {
            let expression = Expression::Unary {
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            dir::UnaryOperator::PostIncrement => unary(UnaryOperator::PostIncrement),
            dir::UnaryOperator::PostDecrement => unary(UnaryOperator::PostDecrement),
            dir::UnaryOperator::PreIncrement => unary(UnaryOperator::PreIncrement),
            dir::UnaryOperator::PreDecrement => unary(UnaryOperator::PreDecrement),
            dir::UnaryOperator::Not => unary(UnaryOperator::Not),
            dir::UnaryOperator::Plus => unary(UnaryOperator::Plus),
            dir::UnaryOperator::Negate => unary(UnaryOperator::Negate),
            dir::UnaryOperator::WrappingNegate => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
            dir::UnaryOperator::ElementwiseNot => unary(UnaryOperator::ElementwiseNot),
            dir::UnaryOperator::Dereference => {
                // nothing to do here
                right_id
            }
            dir::UnaryOperator::Spread => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: None,
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
    ) -> CodegenJsResult<LocalNodeId<Expression>> {
        let left_id = self
            .lower_expression(left_id)
            .expect_node::<Expression>(left_id.into_global_any(self.module.id), self)?;
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<Expression>(right_id.into_global_any(self.module.id), self)?;

        let mut binary = |operator: BinaryOperator| -> LocalNodeId<Expression> {
            let expression = Expression::Binary {
                left: left_id,
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            // multiplication
            dir::BinaryOperator::Multiply => binary(BinaryOperator::Multiply),
            dir::BinaryOperator::Exponent => binary(BinaryOperator::Exponent),
            dir::BinaryOperator::Divide => binary(BinaryOperator::Divide),
            dir::BinaryOperator::Remainder => binary(BinaryOperator::Remainder),

            // addition
            dir::BinaryOperator::Add => binary(BinaryOperator::Add),
            dir::BinaryOperator::Subtract => binary(BinaryOperator::Subtract),

            // shift
            dir::BinaryOperator::ShiftLeft => binary(BinaryOperator::ShiftLeft),
            dir::BinaryOperator::ShiftRight => binary(BinaryOperator::ShiftRight),
            dir::BinaryOperator::UnsignedShiftRight => binary(BinaryOperator::UnsignedShiftRight),

            // elementwise
            dir::BinaryOperator::ElementwiseAnd => binary(BinaryOperator::ElementwiseAnd),
            dir::BinaryOperator::ElementwiseXor => binary(BinaryOperator::ElementwiseXor),
            dir::BinaryOperator::ElementwiseOr => binary(BinaryOperator::ElementwiseOr),

            // comparison
            dir::BinaryOperator::Equal => binary(BinaryOperator::Equal),
            dir::BinaryOperator::NotEqual => binary(BinaryOperator::NotEqual),
            dir::BinaryOperator::EqualStrict => binary(BinaryOperator::EqualStrict),
            dir::BinaryOperator::NotEqualStrict => binary(BinaryOperator::NotEqualStrict),
            dir::BinaryOperator::LessThan => binary(BinaryOperator::LessThan),
            dir::BinaryOperator::LessThanOrEqual => binary(BinaryOperator::LessThanOrEqual),
            dir::BinaryOperator::GreaterThan => binary(BinaryOperator::GreaterThan),
            dir::BinaryOperator::GreaterThanOrEqual => binary(BinaryOperator::GreaterThanOrEqual),

            // boolean
            dir::BinaryOperator::And => binary(BinaryOperator::And),
            dir::BinaryOperator::Or => binary(BinaryOperator::Or),
            dir::BinaryOperator::Coalesce => binary(BinaryOperator::Coalesce),

            // container
            dir::BinaryOperator::In => binary(BinaryOperator::In),
            dir::BinaryOperator::InstanceOf => binary(BinaryOperator::InstanceOf),

            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
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
    ) -> CodegenJsResult<LocalNodeId<Expression>> {
        let left_id = self
            .lower_expression(left_id)
            .expect_node::<Expression>(left_id.into_global_any(self.module.id), self)?;
        let right_id = self
            .lower_expression(right_id)
            .expect_node::<Expression>(right_id.into_global_any(self.module.id), self)?;

        // lower a trivial assign binary expression to a JS assign binary expression
        let mut assign_binary = |operator: AssignOperator| -> LocalNodeId<Expression> {
            let expression = Expression::AssignBinary {
                left: left_id,
                operator,
                right: right_id,
            };
            self.tree
                .insert_from_source(expression, self.module.id, expression_id)
        };

        let expression_id = match operator {
            // assignment multiplication
            dir::AssignOperator::MultiplyAssign => assign_binary(AssignOperator::MultiplyAssign),
            dir::AssignOperator::DivideAssign => assign_binary(AssignOperator::DivideAssign),
            dir::AssignOperator::RemainderAssign => assign_binary(AssignOperator::RemainderAssign),
            dir::AssignOperator::ExponentAssign => assign_binary(AssignOperator::ExponentAssign),

            // assignment addition
            dir::AssignOperator::AddAssign => assign_binary(AssignOperator::AddAssign),
            dir::AssignOperator::SubtractAssign => assign_binary(AssignOperator::SubtractAssign),

            // assignment shift
            dir::AssignOperator::ShiftLeftAssign => assign_binary(AssignOperator::ShiftLeftAssign),
            dir::AssignOperator::ShiftRightAssign => {
                assign_binary(AssignOperator::ShiftRightAssign)
            }
            dir::AssignOperator::UnsignedShiftRightAssign => {
                assign_binary(AssignOperator::UnsignedShiftRightAssign)
            }

            // assignment elementwise
            dir::AssignOperator::ElementwiseAndAssign => {
                assign_binary(AssignOperator::ElementwiseAndAssign)
            }
            dir::AssignOperator::ElementwiseXorAssign => {
                assign_binary(AssignOperator::ElementwiseXorAssign)
            }
            dir::AssignOperator::ElementwiseOrAssign => {
                assign_binary(AssignOperator::ElementwiseOrAssign)
            }

            // assignment boolean
            dir::AssignOperator::AndAssign => assign_binary(AssignOperator::AndAssign),
            dir::AssignOperator::OrAssign => assign_binary(AssignOperator::OrAssign),
            dir::AssignOperator::CoalesceAssign => assign_binary(AssignOperator::CoalesceAssign),

            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };

        Ok(expression_id)
    }
}
