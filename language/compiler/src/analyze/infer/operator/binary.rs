use super::*;
use destack_dir::TypeExpression;

/// Failure categories used to route binary operator diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryOperatorResolutionFailure {
    /// The receiver does not implement the operator contract.
    MissingOperatorContract,
    /// The rhs operand does not satisfy the operator parameter type.
    RhsOperandNotAssignable,
}

/// Diagnostic families emitted for binary operator resolution failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryOperatorFailureDiagnostic {
    /// Report one receiver-centric no-overload diagnostic.
    NoOverloadForReceiver,
    /// Report one rhs operand assignability diagnostic.
    UnassignableOperands,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer one `value is Type` expression.
    pub(crate) fn infer_is_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalNodeId<TypeExpression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve the value type first
        let value_type_id = self.infer_expression(&mut ctx.reborrow(), value_id, state)?;
        let value_type_id = self.unwrap_type_value(value_type_id, ctx.types);

        // resolve the target type from the declared type syntax
        let target_type_id = self.resolve_declared_type_expression(
            &mut ctx.type_context_reborrow(),
            target_type_id,
            true,
            true,
        )?;
        let target_type_id = self.unwrap_type_value(target_type_id, ctx.types);

        // record the runtime check strategy for flow and lowering
        let runtime_check_kind = self.runtime_check_kind_for_relation(
            &mut ctx.type_context_reborrow(),
            value_type_id,
            target_type_id,
        );
        if let Some(kind) = runtime_check_kind {
            ctx.types
                .set_runtime_check_kind(expression_id.into_global_any(ctx.module.id), kind);
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one `value instanceof Target` expression.
    pub(crate) fn infer_instanceof_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve the value and target expression types
        let value_type_id = self.infer_expression(&mut ctx.reborrow(), value_id, state)?;
        let target_type_id = self.infer_expression(&mut ctx.reborrow(), target_id, state)?;
        let value_type_id = self.unwrap_type_value(value_type_id, ctx.types);
        let target_type_id = self.unwrap_type_value(target_type_id, ctx.types);

        // enforce class-only targets in user code
        if matches!(ctx.module.source, ModuleSource::User) {
            let target_symbol =
                self.reference_symbol_for_expression(ctx.tree_symbol_view(), target_id);
            let is_class_target =
                target_symbol.is_some_and(|symbol| symbol.local_id.ty == SymbolType::Class);
            if !is_class_target {
                self.error(AnalyzeError::InvalidInstanceOfTarget {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // record the runtime check strategy for flow and lowering
        let runtime_check_kind = self.runtime_check_kind_for_relation(
            &mut ctx.type_context_reborrow(),
            value_type_id,
            target_type_id,
        );
        if let Some(kind) = runtime_check_kind {
            ctx.types
                .set_runtime_check_kind(expression_id.into_global_any(ctx.module.id), kind);
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    pub(crate) fn infer_binary_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        operator: &BinaryOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the left side first
        let left_ty_id = self.infer_expression(&mut ctx.reborrow(), left_id, state)?;

        // infer the right side after the left
        let right_ty_id = self.infer_expression(&mut ctx.reborrow(), right_id, state)?;

        // load resolved operand types for operator checks
        let left_ty_id = self.unwrap_type_value(left_ty_id, ctx.types);
        let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
        let left_ty = ctx.types.get_type(left_ty_id).clone();
        let right_ty = ctx.types.get_type(right_ty_id).clone();

        // resolve apparent operand types for builtin operator checks
        let left_operator_ty_id = self.normalize_apparent_type(
            &mut ctx.type_context_reborrow(),
            left_ty_id,
            NormalizationMode::Assign,
            RelationMode::OPERATOR_COMPAT,
        );
        let right_operator_ty_id = self.normalize_apparent_type(
            &mut ctx.type_context_reborrow(),
            right_ty_id,
            NormalizationMode::Assign,
            RelationMode::OPERATOR_COMPAT,
        );
        let left_operator_ty = ctx.types.get_type(left_operator_ty_id).clone();
        let right_operator_ty = ctx.types.get_type(right_operator_ty_id).clone();

        // track referential equality violations to avoid follow-up overload errors
        let mut referential_equality_violation = false;

        // reject referential equality when configured
        if state.options.no_referential_equality
            && matches!(ctx.module.source, ModuleSource::User)
            && matches!(
                operator,
                BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::EqualStrict
                    | BinaryOperator::NotEqualStrict
            )
        {
            let left_is_object =
                self.type_is_object_like(&mut ctx.type_context_reborrow(), left_ty_id);
            let right_is_object =
                self.type_is_object_like(&mut ctx.type_context_reborrow(), right_ty_id);
            if left_is_object || right_is_object {
                referential_equality_violation = true;
                self.error(AnalyzeError::ReferentialEqualityDisabled {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // short-circuit when referential equality is forbidden
        if referential_equality_violation {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // guard strict equality against struct types
        if matches!(
            operator,
            BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
        ) {
            let struct_ty_id = if self.is_definitely_struct_type(&left_ty) {
                Some(left_ty_id)
            } else if self.is_definitely_struct_type(&right_ty) {
                Some(right_ty_id)
            } else {
                None
            };

            if let Some(struct_ty_id) = struct_ty_id {
                self.error(AnalyzeError::InvalidStrictEquality {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                    ty: struct_ty_id.into_global(ctx.module.id),
                });
            }
        }

        // handle coalesce operator separately
        if matches!(operator, BinaryOperator::Coalesce) {
            return self.infer_coalesce_expression(
                ctx,
                expression_id,
                left_id,
                right_id,
                left_ty_id,
                right_ty_id,
                &left_ty,
                &right_ty,
                state,
            );
        }

        // handle logical operators with operand unions
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            let result_ty_id =
                self.union_type_from_list(vec![left_ty_id, right_ty_id], left_ty_id, ctx.types);
            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(left_ty_id),
                ctx.infer,
                ctx.types,
            );
            return Ok(result_ty_id);
        }

        // NOTE #Incomplete: full union equality depends on Equal overload dispatch
        // allow literal comparisons when values are assignable
        let is_literal_equality = self.should_use_literal_equality(
            operator,
            &mut ctx.reborrow(),
            left_ty_id,
            right_ty_id,
            left_id,
            right_id,
            &left_ty,
            &right_ty,
        );

        // use builtin rules when appropriate
        let operator_item = operator.language_symbol();
        if self.should_use_builtin_binary_operator(
            operator,
            &left_operator_ty,
            &right_operator_ty,
            ctx.types,
        ) || is_literal_equality
        {
            let ty = self.infer_binary_operation(
                operator,
                &left_operator_ty,
                &right_operator_ty,
                ctx.types,
            );
            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(left_ty_id),
                ctx.infer,
                ctx.types,
            );
            let ty_id = ctx.types.insert_type_from(ty, expression_id);
            self.apply_infer_state_type_freshness(ctx.types, ty_id, state);
            return Ok(ty_id);
        }

        let Some(operator_item) = operator_item else {
            let ty = self.infer_binary_operation(operator, &left_ty, &right_ty, ctx.types);
            let ty_id = ctx.types.insert_type_from(ty, expression_id);
            self.apply_infer_state_type_freshness(ctx.types, ty_id, state);
            return Ok(ty_id);
        };

        // require explicit operator interface implementation
        if !self.is_interface_implemented(ctx.symbol_type_view(), &left_ty, operator_item) {
            let failure_diagnostic = self.binary_operator_failure_diagnostic(
                operator,
                BinaryOperatorResolutionFailure::MissingOperatorContract,
            );
            if failure_diagnostic == BinaryOperatorFailureDiagnostic::UnassignableOperands
                && let Some(error) = self.unassignable_type_error_for_types(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    left_ty_id,
                    right_ty_id,
                )
            {
                return Err(error);
            }

            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                left_ty_id,
            );
            return Ok(self.binary_overload_failure_result_type(expression_id, ctx.types));
        }

        // resolve the operator member function
        let operator_key = self.operator_member_key(operator_item);
        let Some(resolved) = ({
            self.resolve_member_function(
                &mut ctx.reborrow(),
                expression_id,
                left_id,
                Some(left_ty_id),
                &left_ty,
                &operator_key,
            )?
        }) else {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                left_ty_id,
            );
            return Ok(self.binary_overload_failure_result_type(expression_id, ctx.types));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                left_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                left_ty_id,
            );
            return Ok(self.binary_overload_failure_result_type(expression_id, ctx.types));
        }

        // binary operators expect one dynamic parameter
        let parameter_ty_id = resolved.signature.parameters.first().copied();
        if resolved.signature.parameters.len() != 1 {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                left_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                left_ty_id,
            );
            return Ok(self.binary_overload_failure_result_type(expression_id, ctx.types));
        }

        // check argument assignability
        if let Some(parameter_ty_id) = parameter_ty_id {
            let requires_convergence = self.type_relation_requires_infer_convergence(
                ctx.type_view(),
                parameter_ty_id,
                right_ty_id,
            );
            let is_assignable = if requires_convergence {
                true
            } else {
                self.is_type_assignable(
                    &mut ctx.type_context_reborrow(),
                    parameter_ty_id,
                    right_ty_id,
                ) != Assignability::NotAssignable
            };
            if !is_assignable {
                let failure_diagnostic = self.binary_operator_failure_diagnostic(
                    operator,
                    BinaryOperatorResolutionFailure::RhsOperandNotAssignable,
                );
                if failure_diagnostic == BinaryOperatorFailureDiagnostic::NoOverloadForReceiver {
                    self.record_member_call_resolution(
                        &mut ctx.reborrow(),
                        expression_id,
                        left_ty_id,
                        &resolved,
                    )?;
                    self.emit_no_overload_for_receiver_type(
                        ctx.module_type_view(),
                        expression_id.into_any(),
                        left_ty_id,
                    );
                    return Ok(self.binary_overload_failure_result_type(expression_id, ctx.types));
                }

                if let Some(error) = self.unassignable_type_error_for_types(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    parameter_ty_id,
                    right_ty_id,
                ) {
                    self.error(error);
                }

                self.record_member_call_resolution(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_ty_id,
                    &resolved,
                )?;
                return Ok(self.binary_overload_failure_result_type(expression_id, ctx.types));
            }

            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: right_ty_id,
                super_type: parameter_ty_id,
                variance: None,
            });
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(
            &mut ctx.reborrow(),
            expression_id,
            left_ty_id,
            &resolved,
        )?;

        let return_ty_id = resolved.signature.return_type.unwrap_or_else(|| {
            ctx.types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Void,
                },
                expression_id,
            )
        });

        // comparison operators lower compare results to boolean
        if matches!(
            operator,
            BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
        ) {
            let boolean_ty = Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            };
            return Ok(ctx.types.insert_type_from(boolean_ty, expression_id));
        }

        Ok(return_ty_id)
    }

    /// Infer an assignment expression.
    /// Return one result type for operator overload resolution failures.
    fn binary_overload_failure_result_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        types.insert_type_from(Type::Error, expression_id)
    }

    /// Return the diagnostic family for one binary operator resolution failure.
    fn binary_operator_failure_diagnostic(
        &self,
        operator: &BinaryOperator,
        failure: BinaryOperatorResolutionFailure,
    ) -> BinaryOperatorFailureDiagnostic {
        match failure {
            BinaryOperatorResolutionFailure::MissingOperatorContract => {
                if matches!(
                    operator,
                    BinaryOperator::LessThan
                        | BinaryOperator::LessThanOrEqual
                        | BinaryOperator::GreaterThan
                        | BinaryOperator::GreaterThanOrEqual
                ) {
                    BinaryOperatorFailureDiagnostic::UnassignableOperands
                } else {
                    BinaryOperatorFailureDiagnostic::NoOverloadForReceiver
                }
            }
            BinaryOperatorResolutionFailure::RhsOperandNotAssignable => {
                if matches!(
                    operator,
                    BinaryOperator::Add
                        | BinaryOperator::Subtract
                        | BinaryOperator::Multiply
                        | BinaryOperator::Divide
                        | BinaryOperator::Remainder
                        | BinaryOperator::Exponent
                        | BinaryOperator::ElementwiseAnd
                        | BinaryOperator::ElementwiseXor
                        | BinaryOperator::ElementwiseOr
                ) {
                    BinaryOperatorFailureDiagnostic::NoOverloadForReceiver
                } else {
                    BinaryOperatorFailureDiagnostic::UnassignableOperands
                }
            }
        }
    }

    /// Whether the given types and operator have a builtin operator.
    fn should_use_builtin_binary_operator(
        &self,
        operator: &BinaryOperator,
        left_ty: &Type,
        right_ty: &Type,
        types: &TypeTable,
    ) -> bool {
        if operator.language_symbol().is_none() {
            return true;
        }

        if self.operator_operand_is_indeterminate(left_ty, types)
            || self.operator_operand_is_indeterminate(right_ty, types)
        {
            return true;
        }

        match operator {
            BinaryOperator::Add => {
                self.is_string_like_type(left_ty, types)
                    || self.is_string_like_type(right_ty, types)
                    || (self.is_numeric_like_type(left_ty, types)
                        && self.is_numeric_like_type(right_ty, types))
            }
            BinaryOperator::Subtract
            | BinaryOperator::WrappingSubtract
            | BinaryOperator::SaturatingSubtract
            | BinaryOperator::Multiply
            | BinaryOperator::WrappingMultiply
            | BinaryOperator::SaturatingMultiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
            | BinaryOperator::Exponent
            | BinaryOperator::WrappingExponent
            | BinaryOperator::SaturatingExponent
            | BinaryOperator::WrappingAdd
            | BinaryOperator::SaturatingAdd
            | BinaryOperator::ShiftLeft
            | BinaryOperator::SaturatingShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::UnsignedShiftRight
            | BinaryOperator::ElementwiseAnd
            | BinaryOperator::ElementwiseXor
            | BinaryOperator::ElementwiseOr => {
                self.is_numeric_like_type(left_ty, types)
                    && self.is_numeric_like_type(right_ty, types)
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict => {
                // allow pointer comparisons against pointers or nullish values
                let is_pointer = matches!(left_ty, Type::PointerOf { .. })
                    && matches!(
                        right_ty,
                        Type::PointerOf { .. }
                            | Type::TypeLiteral {
                                value: TypeLiteral::Null | TypeLiteral::Undefined,
                            }
                    )
                    || matches!(right_ty, Type::PointerOf { .. })
                        && matches!(
                            left_ty,
                            Type::PointerOf { .. }
                                | Type::TypeLiteral {
                                    value: TypeLiteral::Null | TypeLiteral::Undefined,
                                }
                        );
                if is_pointer {
                    return true;
                }

                self.is_primitive_literal_type(left_ty, types)
                    && self.is_primitive_literal_type(right_ty, types)
            }
            BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => {
                (self.is_numeric_like_type(left_ty, types)
                    && self.is_numeric_like_type(right_ty, types))
                    || (self.is_string_like_type(left_ty, types)
                        && self.is_string_like_type(right_ty, types))
            }
            BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Coalesce
            | BinaryOperator::In => true,
        }
    }

    /// Return a literal type id for a literal value expression.
    fn literal_type_id_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // load the expression node
        let expression = tree.get(expression_id);

        // materialize scalar literal types on demand
        if let Expression::ScalarLiteral { value } = expression {
            let literal_type = self.infer_scalar_literal(value);
            let ty = Type::TypeLiteral {
                value: literal_type,
            };

            return Some(types.insert_type_from(ty, expression_id));
        }

        // materialize nullish literal types on demand
        if let Expression::TypeLiteral { value } = expression
            && matches!(value, TypeLiteral::Null | TypeLiteral::Undefined)
        {
            let ty = Type::TypeLiteral {
                value: value.clone(),
            };

            return Some(types.insert_type_from(ty, expression_id));
        }

        None
    }

    /// Check if equality should use builtin rules for literal comparisons.
    fn should_use_literal_equality(
        &self,
        operator: &BinaryOperator,
        ctx: &mut InferContext<'_>,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        left_ty: &Type,
        right_ty: &Type,
    ) -> bool {
        // only allow equality based literal comparisons
        if !matches!(
            operator,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::EqualStrict
                | BinaryOperator::NotEqualStrict
        ) {
            return false;
        }

        // decide whether either side is a literal value
        let left_literal_type_id = self
            .literal_type_id_for_expression(left_id, ctx.tree, ctx.types)
            .or_else(|| self.is_literal_value_type(left_ty).then_some(left_ty_id));

        let right_literal_type_id = self
            .literal_type_id_for_expression(right_id, ctx.tree, ctx.types)
            .or_else(|| self.is_literal_value_type(right_ty).then_some(right_ty_id));

        // allow comparisons when the left literal is assignable
        if let Some(left_literal_type_id) = left_literal_type_id {
            let assignable = self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                right_ty_id,
                left_literal_type_id,
            );

            return assignable.is_assignable();
        }

        // allow comparisons when the right literal is assignable
        if let Some(right_literal_type_id) = right_literal_type_id {
            let assignable = self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                left_ty_id,
                right_literal_type_id,
            );

            return assignable.is_assignable();
        }

        false
    }
}
