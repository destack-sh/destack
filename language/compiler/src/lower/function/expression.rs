use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError, ScalarType};

use super::{FunctionLowerer, RUNTIME_CHECK_MESSAGES};

impl FunctionLowerer<'_> {
    /// Strip transparent value wrappers around one expression.
    pub(crate) fn unwrap_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> dir::LocalNodeId<dir::Expression> {
        // walk through wrappers that preserve the same runtime base value
        let mut current_id = expression_id;
        loop {
            match self.context.dir_tree.get(current_id) {
                dir::Expression::Parenthesized { expression } => current_id = *expression,
                dir::Expression::As { expression, .. }
                | dir::Expression::Satisfies { expression, .. } => current_id = *expression,
                _ => return current_id,
            }
        }
    }

    /// Resolve one expression as a type literal value.
    pub(crate) fn type_literal_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::TypeLiteral> {
        match self.context.dir_tree.get(expression_id) {
            dir::Expression::Type { value } => match self.context.dir_tree.get(*value) {
                dir::TypeExpression::Literal { value } => Some(value),
                _ => None,
            },
            _ => None,
        }
    }

    /// Lower a binary expression.
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
    pub(super) fn lower_binary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // short-circuit logical operators need special control flow
        if matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            return self.lower_logical_operator(expression_id, operator, left, right);
        }

        // handle union discriminant comparisons
        if let Some(value) =
            self.lower_union_discriminant_comparison(expression_id, left, operator, right)?
        {
            return Ok((value, self.context.type_lowerer.ty_bool));
        }

        // handle union literal comparisons
        if let Some(value) =
            self.lower_union_literal_comparison(expression_id, left, operator, right)?
        {
            return Ok((value, self.context.type_lowerer.ty_bool));
        }

        // handle null reference comparisons
        if let Some(value) =
            self.lower_null_reference_comparison(expression_id, left, operator, right)?
        {
            return Ok((value, self.context.type_lowerer.ty_bool));
        }

        // lower operands and types
        let (mut left_value, left_type) = self.lower_value_expression(left)?;
        let (mut right_value, right_type) = self.lower_value_expression(right)?;
        let result_type = self.lower_type_for_expression(expression_id)?;
        let left_scalar = self.scalar_type_for_mir_type(left_type);
        let right_scalar = self.scalar_type_for_mir_type(right_type);
        let comparison_returns_bool = self.binary_operator_returns_boolean(operator);
        let target_type = if comparison_returns_bool {
            self.common_binary_operand_type(left_type, left_scalar, right_type, right_scalar)
        } else {
            result_type
        };
        let target_scalar = self.scalar_type_for_mir_type(target_type);

        // coerce numeric operands to the result type when needed
        if let Some(target_scalar) = target_scalar
            && self.is_numeric_scalar_type(target_scalar)
            && let Some(left_scalar) = left_scalar
            && let Some(right_scalar) = right_scalar
            && self.is_numeric_scalar_type(left_scalar)
            && self.is_numeric_scalar_type(right_scalar)
        {
            left_value = self.cast_numeric_value(
                expression_id,
                left_value,
                left_scalar,
                target_scalar,
                target_type,
            )?;
            right_value = self.cast_numeric_value(
                expression_id,
                right_value,
                right_scalar,
                target_scalar,
                target_type,
            )?;
        }

        // emit checked integer arithmetic when configured
        if self.overflow_checks_enabled()
            && matches!(
                operator,
                dir::BinaryOperator::Add
                    | dir::BinaryOperator::Subtract
                    | dir::BinaryOperator::Multiply
            )
            && let Some((_, is_signed)) = self.integer_scalar_info(expression_id)?
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
            && let Some((width, is_signed)) = self.integer_scalar_info(expression_id)?
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
            && let Some((left_width, _)) = self.integer_scalar_info(expression_id)?
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
        let operator_scalar = target_scalar
            .ok_or_else(|| self.missing_type_error(expression_id))
            .map_err(CompilerError::from)?;
        let op = self.lower_binary_operator(expression_id, operator, operator_scalar)?;
        let value = self.state.builder.binary_op(op, left_value, right_value);

        // comparisons produce bool, others preserve operand type
        let ty = if op.is_comparison() {
            self.context.type_lowerer.ty_bool
        } else {
            result_type
        };

        Ok((value, ty))
    }

    /// Check whether a binary operator returns a boolean value.
    fn binary_operator_returns_boolean(&self, operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::EqualStrict
                | dir::BinaryOperator::NotEqualStrict
                | dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        )
    }

    /// Choose the machine operand type for one binary operation.
    fn common_binary_operand_type(
        &self,
        left_type: mir::LocalNodeId<mir::Type>,
        left_scalar: Option<ScalarType>,
        right_type: mir::LocalNodeId<mir::Type>,
        right_scalar: Option<ScalarType>,
    ) -> mir::LocalNodeId<mir::Type> {
        let Some(left_scalar) = left_scalar else {
            return left_type;
        };
        let Some(right_scalar) = right_scalar else {
            return left_type;
        };

        if !self.is_numeric_scalar_type(left_scalar) || !self.is_numeric_scalar_type(right_scalar) {
            return left_type;
        }

        if self.numeric_scalar_rank(right_scalar) > self.numeric_scalar_rank(left_scalar) {
            right_type
        } else {
            left_type
        }
    }

    /// Lower one runtime type guard expression.
    pub(super) fn lower_runtime_type_guard_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        target_type_id: dir::LocalTypeId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // require runtime check metadata from Analyze
        let guard_entry = self
            .context
            .guards
            .entry(expression_id.into_global_any(self.context.module_id));
        let Some(guard_entry) = guard_entry else {
            return Err(LowerError::Internal {
                anchor: (self.context.module_id).into(),
                module: self.context.module_id,
                message: "missing runtime check metadata for type guard".to_string(),
            }
            .into());
        };

        // handle constant guards early
        if let dir::GuardEntry::Constant(value) = guard_entry {
            let value = self.state.builder.bconst(value);
            return Ok((value, self.context.type_lowerer.ty_bool));
        }

        // reject type descriptor guards until RTTI is lowered (#Incomplete)
        if guard_entry == dir::GuardEntry::TypeDescriptor {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "type descriptor checks are not lowered yet".to_string(),
            }
            .into());
        }

        // resolve expression and target types
        let (left_value, left_mir_type) = self.lower_value_expression(left)?;
        let left_type_id = self.type_for_expression_or_error(left)?;
        let left_type_id = self.unwrap_value_type_id(left_type_id);

        let target_type_id = self.unwrap_value_type_id(target_type_id);

        // exact or nominally equivalent matches are always true
        if self.are_type_ids_equivalent(left_type_id, target_type_id) {
            let value = self.state.builder.bconst(true);
            return Ok((value, self.context.type_lowerer.ty_bool));
        }

        // union types use the tag field for runtime checks
        let is_union_value = matches!(
            self.context.types.get_type(left_type_id),
            dir::Type::Union(_)
        );
        if guard_entry == dir::GuardEntry::UnionTag && !is_union_value {
            return Err(LowerError::Internal {
                anchor: (self.context.module_id).into(),
                module: self.context.module_id,
                message: "runtime check metadata expected union value".to_string(),
            }
            .into());
        }
        if is_union_value {
            let layout = self
                .context
                .type_lowerer
                .union_layout(left_type_id)
                .ok_or_else(|| self.missing_type_error(expression_id))
                .map_err(CompilerError::from)?;

            // resolve the tag for the target type
            let Some(tag_index) = layout
                .source_types
                .iter()
                .position(|element| self.are_type_ids_equivalent(*element, target_type_id))
            else {
                let value = self.state.builder.bconst(false);
                return Ok((value, self.context.type_lowerer.ty_bool));
            };

            // build the tag constant
            let (tag_width, tag_signed) = match self.state.builder.tree().get(layout.tag_type) {
                mir::Type::Int {
                    width,
                    is_signed: signed,
                } => (*width, *signed),
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "union tag must be an integer type".to_string(),
                    }
                    .into());
                }
            };
            let tag_const = self
                .state
                .builder
                .iconst(tag_index as i128, tag_width, tag_signed);

            // load through references before extracting the tag
            let union_value = match self.state.builder.tree().get(left_mir_type) {
                mir::Type::Reference { pointee, .. } => {
                    let pointee = pointee.ty().ok_or_else(|| {
                        self.error(expression_id, "union reference pointee is not concrete")
                    })?;

                    self.state.builder.load(left_value, pointee)
                }
                _ => left_value,
            };
            let tag_value = self
                .state
                .builder
                .field_get(union_value, layout.tag_field_index);

            // emit the tag comparison
            let cmp =
                self.state
                    .builder
                    .binary_op(mir::BinaryOperator::Equal, tag_value, tag_const);
            return Ok((cmp, self.context.type_lowerer.ty_bool));
        }

        Err(LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: "unsupported type check".to_string(),
        }
        .into())
    }

    /// Resolve the target type id for one `is` guard.
    pub(crate) fn is_target_type_id(
        &self,
        expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::LocalTypeId> {
        self.type_id_for_type_expression(expression_id)
            .ok_or_else(|| {
                self.missing_type_error_for_node(
                    expression_id.into_global_any(self.context.module_id),
                )
            })
            .map_err(CompilerError::from)
    }

    /// Check whether two type ids refer to the same nominal type.
    fn are_type_ids_equivalent(
        &self,
        left_type_id: dir::LocalTypeId,
        right_type_id: dir::LocalTypeId,
    ) -> bool {
        self.type_ids_equivalent(left_type_id, right_type_id)
    }

    /// Check whether a scalar type is numeric.
    fn is_numeric_scalar_type(&self, scalar_type: ScalarType) -> bool {
        matches!(
            scalar_type,
            ScalarType::SignedInt { .. }
                | ScalarType::UnsignedInt { .. }
                | ScalarType::Float { .. }
        )
    }

    /// Return a coarse numeric scalar rank for operation selection.
    fn numeric_scalar_rank(&self, scalar_type: ScalarType) -> u8 {
        match scalar_type {
            ScalarType::Float { .. } => 3,
            ScalarType::SignedInt { .. } | ScalarType::UnsignedInt { .. } => 2,
            ScalarType::Bool => 0,
        }
    }

    /// Resolve a scalar type for a MIR type id.
    pub(crate) fn scalar_type_for_mir_type(
        &self,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> Option<ScalarType> {
        let mir_type = self.state.builder.tree().get(type_id);
        match mir_type {
            mir::Type::Int { width, is_signed } => Some(if *is_signed {
                ScalarType::SignedInt { width: *width }
            } else {
                ScalarType::UnsignedInt { width: *width }
            }),
            mir::Type::Isize => Some(ScalarType::SignedInt {
                width: self.context.type_lowerer.pointer_width_bits(),
            }),
            mir::Type::Usize => Some(ScalarType::UnsignedInt {
                width: self.context.type_lowerer.pointer_width_bits(),
            }),
            mir::Type::Float(float_type) => Some(ScalarType::Float {
                width: float_type.width(),
            }),
            _ => None,
        }
    }

    /// Cast a numeric value to the target scalar type when needed.
    fn cast_numeric_value(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        source: ScalarType,
        target: ScalarType,
        target_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        if source == target {
            return Ok(value);
        }

        let operator = match (source, target) {
            (ScalarType::SignedInt { width: from }, ScalarType::SignedInt { width: to }) => {
                if to > from {
                    mir::CastOperator::SignExtend
                } else if to < from {
                    mir::CastOperator::Truncate
                } else {
                    return Ok(value);
                }
            }
            (ScalarType::UnsignedInt { width: from }, ScalarType::UnsignedInt { width: to }) => {
                if to > from {
                    mir::CastOperator::ZeroExtend
                } else if to < from {
                    mir::CastOperator::Truncate
                } else {
                    return Ok(value);
                }
            }
            (ScalarType::SignedInt { .. }, ScalarType::UnsignedInt { .. })
            | (ScalarType::UnsignedInt { .. }, ScalarType::SignedInt { .. }) => {
                mir::CastOperator::Bitcast
            }
            (ScalarType::SignedInt { .. }, ScalarType::Float { .. }) => {
                mir::CastOperator::SignedIntToFloat
            }
            (ScalarType::UnsignedInt { .. }, ScalarType::Float { .. }) => {
                mir::CastOperator::UnsignedIntToFloat
            }
            (ScalarType::Float { .. }, ScalarType::SignedInt { .. }) => {
                mir::CastOperator::FloatToSignedInt
            }
            (ScalarType::Float { .. }, ScalarType::UnsignedInt { .. }) => {
                mir::CastOperator::FloatToUnsignedInt
            }
            (ScalarType::Float { width: from }, ScalarType::Float { width: to }) => {
                if to > from {
                    mir::CastOperator::FloatExtend
                } else if to < from {
                    mir::CastOperator::FloatTruncate
                } else {
                    return Ok(value);
                }
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported numeric cast in binary expression".to_string(),
                }
                .into());
            }
        };

        Ok(self.state.builder.cast(operator, value, target_type))
    }

    /// Lower null comparisons against references that allow null.
    fn lower_null_reference_comparison(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        // only handle equality comparisons
        if !matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::EqualStrict
                | dir::BinaryOperator::NotEqualStrict
        ) {
            return Ok(None);
        }

        // unwrap implicit casts and parens before matching
        let left = self.unwrap_expression(left);
        let right = self.unwrap_expression(right);

        // match null comparisons
        let (value_id, literal) = match (
            self.type_literal_for_expression(left),
            self.type_literal_for_expression(right),
        ) {
            (Some(value), _) => (right, value),
            (_, Some(value)) => (left, value),
            _ => return Ok(None),
        };

        // only handle null literal comparisons here
        if !matches!(literal, dir::TypeLiteral::Null) {
            return Ok(None);
        }

        // lower the operand and confirm it allows null
        let (value, value_type) = self.lower_value_expression(value_id)?;
        let is_null_reference = matches!(
            self.state.builder.tree().get(value_type),
            mir::Type::Reference {
                nullability: mir::Nullability::Null | mir::Nullability::NullOrUndefined,
                ..
            } | mir::Type::TensorView {
                nullability: mir::Nullability::Null | mir::Nullability::NullOrUndefined,
                ..
            }
        );
        if !is_null_reference {
            return Ok(None);
        }

        // build the null literal
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let null_value = self.zero_value_for_type(value_type, node)?;

        // emit the comparison
        let op = if matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
        ) {
            mir::BinaryOperator::Equal
        } else {
            mir::BinaryOperator::NotEqual
        };
        let value = self.state.builder.binary_op(op, value, null_value);

        Ok(Some(value))
    }

    /// Lower overflow-checked integer arithmetic.
    fn lower_overflow_checked_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left_value: mir::Value,
        right_value: mir::Value,
        is_signed: bool,
    ) -> CompilerResult<mir::Value> {
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
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported overflow operator".to_string(),
                }
                .into());
            }
        };

        // build the intrinsic result tuple type
        let result_type = self.lower_type_for_expression(expression_id)?;
        let bool_type = self.state.builder.tree_mut().boolean_type();
        let result_copyability = self
            .state
            .builder
            .tree()
            .get(result_type)
            .copy()
            .combine(mir::Copy::Yes);
        let pair_type = self.state.builder.tree_mut().insert_type(mir::Type::Tuple {
            elements: vec![result_type.into(), bool_type.into()],
            copy: result_copyability,
        });

        // emit the checked intrinsic
        let pair =
            self.state
                .builder
                .intrinsic(intrinsic, pair_type, vec![left_value, right_value]);
        let result = self.state.builder.field_get(pair, 0);
        let _overflow = self.state.builder.field_get(pair, 1);

        // emit the overflow check
        let constraint = mir::CheckConstraint::Overflow {
            operator: constraint_operator,
            left: left_value.into(),
            right: right_value.into(),
            is_signed,
        };
        self.emit_check(constraint, RUNTIME_CHECK_MESSAGES.integer_overflow)?;

        Ok(result)
    }

    /// Emit division checks for integer division and remainder.
    fn lower_division_checked_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left_value: mir::Value,
        right_value: mir::Value,
        width: u16,
        is_signed: bool,
    ) -> CompilerResult<()> {
        // ensure division operators are used here
        if !matches!(
            operator,
            dir::BinaryOperator::Divide | dir::BinaryOperator::Remainder
        ) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "unsupported division operator".to_string(),
            }
            .into());
        }

        // build the div-zero check
        let constraint = mir::CheckConstraint::DivZero {
            divisor: right_value.into(),
        };
        self.emit_check(constraint, RUNTIME_CHECK_MESSAGES.division_by_zero)?;

        // emit signed min / -1 overflow checks
        if is_signed {
            let _min_value = match width {
                8 => i64::from(i8::MIN),
                16 => i64::from(i16::MIN),
                32 => i64::from(i32::MIN),
                64 => i64::MIN,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported integer width for overflow checks".to_string(),
                    }
                    .into());
                }
            };
            let constraint = mir::CheckConstraint::Overflow {
                operator: mir::BinaryOperator::SignedDivide,
                left: left_value.into(),
                right: right_value.into(),
                is_signed,
            };
            self.emit_check(constraint, RUNTIME_CHECK_MESSAGES.division_overflow)?;
        }

        Ok(())
    }

    /// Emit shift checks for integer shift operations.
    fn lower_shift_checked_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right_value: mir::Value,
        left_width: u16,
        _shift_width: u16,
        shift_signed: bool,
    ) -> CompilerResult<()> {
        // ensure shift operators are used here
        if !matches!(
            operator,
            dir::BinaryOperator::ShiftLeft
                | dir::BinaryOperator::ShiftRight
                | dir::BinaryOperator::UnsignedShiftRight
        ) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "unsupported shift operator".to_string(),
            }
            .into());
        }

        let bit_width = u8::try_from(left_width).map_err(|_| LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: "shift width exceeds check constraint limits".to_string(),
        })?;
        let constraint = mir::CheckConstraint::ShiftRange {
            value: right_value.into(),
            bit_width,
            is_signed: shift_signed,
        };
        self.emit_check(constraint, RUNTIME_CHECK_MESSAGES.shift_out_of_range)?;

        Ok(())
    }

    /// Lower a conditional (ternary) expression.
    ///
    /// ```ds
    /// function pick(flag: boolean): int32 {
    ///     return flag ? 1 : 2;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// bb0(v0: boolean):
    ///     branch v0, bb1, bb2
    /// bb1:
    ///     v1: int32 = const 1
    ///     jump bb3(v1)
    /// bb2:
    ///     v2: int32 = const 2
    ///     jump bb3(v2)
    /// bb3(v3: int32):
    ///     return v3
    /// ```
    pub(crate) fn lower_conditional_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        condition_id: dir::LocalNodeId<dir::Expression>,
        then_id: dir::LocalNodeId<dir::Expression>,
        else_id: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // require else branch for value expressions
        let else_id = else_id
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "conditional expression requires else branch".to_string(),
            })
            .map_err(CompilerError::from)?;

        // get the result type from the expression
        let result_type = self.lower_type_for_expression(expression_id)?;

        // create blocks for each branch
        let then_block = self.state.builder.block();
        let else_block = self.state.builder.block();
        let merge_block = self.state.builder.block();

        // create a variable to hold the result
        let result_variable = self.state.builder.variable(result_type);

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
