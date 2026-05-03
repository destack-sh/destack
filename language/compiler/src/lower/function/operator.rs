use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerResult, LowerError, ScalarType};

use crate::lower::FunctionLowerer;

impl FunctionLowerer<'_> {
    /// Lower a binary operator.
    ///
    /// ```ds
    /// function add(a: int32, b: int32): int32 {
    ///     return a + b;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v2: i32 = binary.add v0, v1
    /// ```
    pub(crate) fn lower_binary_operator(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        scalar_type: ScalarType,
    ) -> CompilerResult<mir::BinaryOperator> {
        let is_float = matches!(scalar_type, ScalarType::Float { .. });
        let is_signed = matches!(scalar_type, ScalarType::SignedInt { .. });

        let op = match (operator, is_float, is_signed) {
            // integer arithmetic
            (dir::BinaryOperator::Add, false, _) => mir::BinaryOperator::Add,
            (dir::BinaryOperator::Subtract, false, _) => mir::BinaryOperator::Subtract,
            (dir::BinaryOperator::Multiply, false, _) => mir::BinaryOperator::Multiply,
            (dir::BinaryOperator::Divide, false, true) => mir::BinaryOperator::SignedDivide,
            (dir::BinaryOperator::Divide, false, false) => mir::BinaryOperator::UnsignedDivide,
            (dir::BinaryOperator::Remainder, false, true) => mir::BinaryOperator::SignedRemainder,
            (dir::BinaryOperator::Remainder, false, false) => {
                mir::BinaryOperator::UnsignedRemainder
            }

            // float arithmetic
            (dir::BinaryOperator::Add, true, _) => mir::BinaryOperator::FloatAdd,
            (dir::BinaryOperator::Subtract, true, _) => mir::BinaryOperator::FloatSubtract,
            (dir::BinaryOperator::Multiply, true, _) => mir::BinaryOperator::FloatMultiply,
            (dir::BinaryOperator::Divide, true, _) => mir::BinaryOperator::FloatDivide,

            // bitwise operators (integers only)
            (dir::BinaryOperator::ElementwiseAnd, false, _) => mir::BinaryOperator::And,
            (dir::BinaryOperator::ElementwiseOr, false, _) => mir::BinaryOperator::Or,
            (dir::BinaryOperator::ElementwiseXor, false, _) => mir::BinaryOperator::Xor,

            // shift operators (integers only)
            (dir::BinaryOperator::ShiftLeft, false, _) => mir::BinaryOperator::ShiftLeft,
            (dir::BinaryOperator::ShiftRight, false, _) => {
                mir::BinaryOperator::ArithmeticShiftRight
            }
            (dir::BinaryOperator::UnsignedShiftRight, false, _) => {
                mir::BinaryOperator::LogicalShiftRight
            }

            // integer comparison
            (dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict, false, _) => {
                mir::BinaryOperator::Equal
            }
            (dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict, false, _) => {
                mir::BinaryOperator::NotEqual
            }
            (dir::BinaryOperator::LessThan, false, true) => mir::BinaryOperator::SignedLessThan,
            (dir::BinaryOperator::LessThanOrEqual, false, true) => {
                mir::BinaryOperator::SignedLessEqual
            }
            (dir::BinaryOperator::GreaterThan, false, true) => {
                mir::BinaryOperator::SignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThanOrEqual, false, true) => {
                mir::BinaryOperator::SignedGreaterEqual
            }
            (dir::BinaryOperator::LessThan, false, false) => mir::BinaryOperator::UnsignedLessThan,
            (dir::BinaryOperator::LessThanOrEqual, false, false) => {
                mir::BinaryOperator::UnsignedLessEqual
            }
            (dir::BinaryOperator::GreaterThan, false, false) => {
                mir::BinaryOperator::UnsignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThanOrEqual, false, false) => {
                mir::BinaryOperator::UnsignedGreaterEqual
            }

            // float comparison
            (dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict, true, _) => {
                mir::BinaryOperator::FloatEqual
            }
            (dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict, true, _) => {
                mir::BinaryOperator::FloatNotEqual
            }
            (dir::BinaryOperator::LessThan, true, _) => mir::BinaryOperator::FloatLessThan,
            (dir::BinaryOperator::LessThanOrEqual, true, _) => mir::BinaryOperator::FloatLessEqual,
            (dir::BinaryOperator::GreaterThan, true, _) => mir::BinaryOperator::FloatGreaterThan,
            (dir::BinaryOperator::GreaterThanOrEqual, true, _) => {
                mir::BinaryOperator::FloatGreaterEqual
            }

            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: format!("unsupported binary operator '{operator:?}'"),
                }
                .into());
            }
        };

        Ok(op)
    }

    /// Lower a unary operator.
    ///
    /// ```ds
    /// function neg(value: int32): int32 {
    ///     return -value;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: i32 = unary.neg v0
    /// ```
    pub(crate) fn lower_unary_operator(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        operand_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::UnaryOperator> {
        let scalar_type = self.scalar_type_for_expression(operand_id);
        let is_float = matches!(scalar_type, Some(ScalarType::Float { .. }));

        let op = match (operator, is_float) {
            // negation
            (dir::UnaryOperator::Negate, false) => mir::UnaryOperator::Negate,
            (dir::UnaryOperator::Negate, true) => mir::UnaryOperator::FloatNegate,

            // bitwise NOT (~) and logical NOT (!) both use the same MIR op
            (dir::UnaryOperator::ElementwiseNot, _) | (dir::UnaryOperator::Not, _) => {
                mir::UnaryOperator::Not
            }

            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: format!("unsupported unary operator '{operator:?}'"),
                }
                .into());
            }
        };

        Ok(op)
    }

    /// Lower a short-circuit logical operator (&& or ||).
    ///
    /// Short-circuit evaluation means:
    /// - `a && b`: if `a` is false, result is false without evaluating `b`
    /// - `a || b`: if `a` is true, result is true without evaluating `b`
    ///
    /// ```ds
    /// function both(a: boolean, b: boolean): boolean {
    ///     return a && b;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// branch v0, block1, block2
    /// block1:
    ///     branch v1, block3, block2
    /// block2:
    ///     v2: bool = bconst false
    /// ```
    pub(crate) fn lower_logical_operator(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left_id: dir::LocalNodeId<dir::Expression>,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // evaluate LHS first
        let (lhs_value, lhs_type) = self.lower_value_expression(left_id)?;

        // verify LHS is boolean
        if lhs_type != self.context.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "logical operator requires boolean operands".to_string(),
            }
            .into());
        }

        // create blocks for short-circuit evaluation
        let shortcircuit_block = self.state.builder.block();
        let rhs_block = self.state.builder.block();
        let merge_block = self.state.builder.block();

        // create a variable to hold the result (SSA construction will merge)
        let result_variable = self
            .state
            .builder
            .variable(self.context.type_lowerer.ty_bool);

        // branch based on operator semantics
        match operator {
            dir::BinaryOperator::And => {
                // a && b: if a is true, evaluate b; else short-circuit to false
                self.state
                    .builder
                    .branch(lhs_value, rhs_block, shortcircuit_block);
            }
            dir::BinaryOperator::Or => {
                // a || b: if a is true, short-circuit to true; else evaluate b
                self.state
                    .builder
                    .branch(lhs_value, shortcircuit_block, rhs_block);
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: format!("unexpected logical operator '{operator:?}'"),
                }
                .into());
            }
        }

        // short-circuit block: set result to constant and jump to merge
        self.state.builder.switch_to_block(shortcircuit_block);
        let shortcircuit_value = match operator {
            dir::BinaryOperator::And => self.state.builder.bconst(false),
            dir::BinaryOperator::Or => self.state.builder.bconst(true),
            _ => unreachable!(),
        };
        self.state
            .builder
            .define_variable(result_variable, shortcircuit_value);
        self.state.builder.jump(merge_block);

        // rhs block: evaluate rhs, set result, jump to merge
        self.state.builder.switch_to_block(rhs_block);
        let (rhs_value, rhs_type) = self.lower_value_expression(right_id)?;

        // verify RHS is boolean
        if rhs_type != self.context.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "logical operator requires boolean operands".to_string(),
            }
            .into());
        }

        // set result and jump to merge
        self.state
            .builder
            .define_variable(result_variable, rhs_value);
        self.state.builder.jump(merge_block);

        // merge block: use the result variable (SSA will create block parameter)
        self.state.builder.switch_to_block(merge_block);
        let result_value = self.state.builder.use_variable(result_variable);

        Ok((result_value, self.context.type_lowerer.ty_bool))
    }
}
