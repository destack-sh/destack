use destack_dir::{Expression, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::FunctionContext;

// check failure messages
const DIVISION_CHECK_MESSAGE: &str = "division by zero";
const DIVISION_OVERFLOW_MESSAGE: &str = "division overflow";
const OVERFLOW_CHECK_MESSAGE: &str = "integer overflow";
const SHIFT_CHECK_MESSAGE: &str = "shift out of range";

impl FunctionContext<'_> {
    /// Lower a binary expression.
    pub(super) fn lower_binary_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // short-circuit logical operators need special control flow
        if matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            return self.lower_logical_operator(expression_id, operator, left, right);
        }

        // handle union discriminant comparisons
        if let Some(value) =
            self.lower_union_discriminant_comparison(expression_id, left, operator, right)?
        {
            return Ok((value, self.env.type_lowerer.ty_bool));
        }

        // lower operands and types
        let (left_value, _) = self.lower_value_expression(left)?;
        let (right_value, _) = self.lower_value_expression(right)?;
        let result_type = self.lower_type_for_expression(expression_id)?;
        let integer_info = self.integer_scalar_info(left)?;

        // emit checked integer arithmetic when configured
        if self.overflow_checks_enabled()
            && matches!(
                operator,
                dir::BinaryOperator::Add
                    | dir::BinaryOperator::Subtract
                    | dir::BinaryOperator::Multiply
            )
            && let Some((_, is_signed)) = integer_info
        {
            let value = self.lower_overflow_checked_binary(
                expression_id,
                operator,
                left_value,
                right_value,
                is_signed,
            )?;
            return Ok((value, result_type));
        }

        // emit division checks when configured
        if self.division_checks_enabled()
            && matches!(
                operator,
                dir::BinaryOperator::Divide | dir::BinaryOperator::Remainder
            )
            && let Some((width, is_signed)) = integer_info
        {
            self.lower_division_checked_binary(
                expression_id,
                operator,
                left_value,
                right_value,
                width,
                is_signed,
            )?;
        }

        // emit shift checks when configured
        if self.shift_checks_enabled()
            && matches!(
                operator,
                dir::BinaryOperator::ShiftLeft
                    | dir::BinaryOperator::ShiftRight
                    | dir::BinaryOperator::UnsignedShiftRight
            )
            && let Some((left_width, _)) = integer_info
            && let Some((shift_width, shift_signed)) = self.integer_scalar_info(right)?
        {
            self.lower_shift_checked_binary(
                expression_id,
                operator,
                right_value,
                left_width,
                shift_width,
                shift_signed,
            )?;
        }

        // emit binary operation
        let op = self.lower_binary_operator(expression_id, operator, left)?;
        let value = self.state.builder.binary_op(op, left_value, right_value);

        // comparisons produce bool, others preserve operand type
        let ty = if op.is_comparison() {
            self.env.type_lowerer.ty_bool
        } else {
            result_type
        };

        Ok((value, ty))
    }

    /// Lower overflow-checked integer arithmetic.
    fn lower_overflow_checked_binary(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        left_value: mir::Value,
        right_value: mir::Value,
        is_signed: bool,
    ) -> LowerResult<mir::Value> {
        // select the checked intrinsic for this operator
        let (intrinsic, constraint_operator) = match operator {
            dir::BinaryOperator::Add => (mir::Intrinsic::AddOverflow, mir::BinaryOperator::Add),
            dir::BinaryOperator::Subtract => {
                (mir::Intrinsic::SubOverflow, mir::BinaryOperator::Subtract)
            }
            dir::BinaryOperator::Multiply => {
                (mir::Intrinsic::MulOverflow, mir::BinaryOperator::Multiply)
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "unsupported overflow operator".to_string(),
                });
            }
        };

        // emit the checked intrinsic
        let pair = self
            .state
            .builder
            .intrinsic(intrinsic, vec![left_value, right_value]);
        let result = self.state.builder.field_get(pair, 0);
        let overflow = self.state.builder.field_get(pair, 1);

        // emit the overflow check
        let condition = self.state.builder.bnot(overflow);
        let constraint = mir::CheckConstraint::Overflow {
            operator: constraint_operator,
            left: left_value,
            right: right_value,
            is_signed,
        };
        self.emit_check(condition, constraint, OVERFLOW_CHECK_MESSAGE);

        Ok(result)
    }

    /// Emit division checks for integer division and remainder.
    fn lower_division_checked_binary(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        left_value: mir::Value,
        right_value: mir::Value,
        width: u16,
        is_signed: bool,
    ) -> LowerResult<()> {
        // ensure division operators are used here
        if !matches!(
            operator,
            dir::BinaryOperator::Divide | dir::BinaryOperator::Remainder
        ) {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "unsupported division operator".to_string(),
            });
        }

        // build the div-zero check
        let zero = self.state.builder.iconst(0, width as u8, is_signed);
        let condition =
            self.state
                .builder
                .binary_op(mir::BinaryOperator::NotEqual, right_value, zero);
        let constraint = mir::CheckConstraint::DivZero {
            divisor: right_value,
        };
        self.emit_check(condition, constraint, DIVISION_CHECK_MESSAGE);

        // emit signed min / -1 overflow checks
        if is_signed {
            let min_value = match width {
                8 => i64::from(i8::MIN),
                16 => i64::from(i16::MIN),
                32 => i64::from(i32::MIN),
                64 => i64::MIN,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "unsupported integer width for overflow checks".to_string(),
                    });
                }
            };
            let min_const = self.state.builder.iconst(min_value, width as u8, true);
            let neg_one = self.state.builder.iconst(-1, width as u8, true);
            let left_is_min =
                self.state
                    .builder
                    .binary_op(mir::BinaryOperator::Equal, left_value, min_const);
            let right_is_neg_one =
                self.state
                    .builder
                    .binary_op(mir::BinaryOperator::Equal, right_value, neg_one);
            let overflow = self.state.builder.binary_op(
                mir::BinaryOperator::And,
                left_is_min,
                right_is_neg_one,
            );
            let condition = self.state.builder.bnot(overflow);
            let constraint = mir::CheckConstraint::Overflow {
                operator: mir::BinaryOperator::SignedDivide,
                left: left_value,
                right: right_value,
                is_signed,
            };
            self.emit_check(condition, constraint, DIVISION_OVERFLOW_MESSAGE);
        }

        Ok(())
    }

    /// Emit shift checks for integer shift operations.
    fn lower_shift_checked_binary(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        right_value: mir::Value,
        left_width: u16,
        shift_width: u16,
        shift_signed: bool,
    ) -> LowerResult<()> {
        // ensure shift operators are used here
        if !matches!(
            operator,
            dir::BinaryOperator::ShiftLeft
                | dir::BinaryOperator::ShiftRight
                | dir::BinaryOperator::UnsignedShiftRight
        ) {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "unsupported shift operator".to_string(),
            });
        }

        let bit_width = u8::try_from(left_width).map_err(|_| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
            message: "shift width exceeds check constraint limits".to_string(),
        })?;
        let shift_width =
            u8::try_from(shift_width).map_err(|_| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "shift amount width exceeds check constraint limits".to_string(),
            })?;
        let bit_width_value =
            self.state
                .builder
                .iconst(bit_width as i64, shift_width, shift_signed);
        let condition = if shift_signed {
            let zero = self.state.builder.iconst(0, shift_width, true);
            let non_negative = self.state.builder.binary_op(
                mir::BinaryOperator::SignedGreaterEqual,
                right_value,
                zero,
            );
            let in_range = self.state.builder.binary_op(
                mir::BinaryOperator::SignedLessThan,
                right_value,
                bit_width_value,
            );
            self.state
                .builder
                .binary_op(mir::BinaryOperator::And, non_negative, in_range)
        } else {
            self.state.builder.binary_op(
                mir::BinaryOperator::UnsignedLessThan,
                right_value,
                bit_width_value,
            )
        };
        let constraint = mir::CheckConstraint::ShiftRange {
            value: right_value,
            bit_width,
            is_signed: shift_signed,
        };
        self.emit_check(condition, constraint, SHIFT_CHECK_MESSAGE);

        Ok(())
    }

    /// Lower a conditional (ternary) expression.
    pub(crate) fn lower_conditional_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        condition_id: LocalNodeId<Expression>,
        then_id: LocalNodeId<Expression>,
        else_id: Option<LocalNodeId<Expression>>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // require else branch for value expressions
        let else_id = else_id.ok_or_else(|| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
            message: "conditional expression requires else branch".to_string(),
        })?;

        // get the result type from the expression
        let result_type = self.lower_type_for_expression(expression_id)?;

        // create blocks for each branch
        let then_block = self.state.builder.create_block();
        let else_block = self.state.builder.create_block();
        let merge_block = self.state.builder.create_block();

        // create a variable to hold the result
        let result_variable = self.state.builder.create_variable(result_type);

        // branch based on condition (with union tag checks when possible)
        let did_check = self.lower_union_tag_check(condition_id, then_block, else_block)?;
        if !did_check {
            // evaluate condition first
            let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
            self.check_type_is_bool(condition_id, condition_type, "conditional expression")?;

            // branch
            self.state
                .builder
                .branch(condition_value, then_block, else_block);
        }

        // then block: evaluate then expression and set result
        self.state.builder.switch_to_block(then_block);
        let (then_value, _) = self.lower_value_expression(then_id)?;
        self.state
            .builder
            .define_variable(result_variable, then_value);
        self.state.builder.jump(merge_block);

        // else block: evaluate else expression and set result
        self.state.builder.switch_to_block(else_block);
        let (else_value, _) = self.lower_value_expression(else_id)?;
        self.state
            .builder
            .define_variable(result_variable, else_value);
        self.state.builder.jump(merge_block);

        // merge block: use the result variable (SSA will create block parameter)
        self.state.builder.switch_to_block(merge_block);
        let result_value = self.state.builder.use_variable(result_variable);

        Ok((result_value, result_type))
    }
}
