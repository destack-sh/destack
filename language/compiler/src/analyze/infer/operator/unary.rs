use super::*;
#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_unary_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        operator: &UnaryOperator,
        right_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_OPERATOR);

        let mut right_ctx = state.fork().with_expected_type(None);
        let right_ty_id = self.infer_expression(&mut ctx.reborrow(), right_id, &mut right_ctx)?;
        let right_ty = ctx.types.get_type(right_ty_id).clone();

        let operator_item = operator.language_symbol();

        // use builtin rules when appropriate
        if self.should_use_builtin_unary_operator(operator, &right_ty, ctx.types) {
            if matches!(operator, UnaryOperator::Dereference)
                && let Type::PointerOf { right, .. } = &right_ty
            {
                self.record_provisional_builtin_resolution(
                    expression_id.into_global_any(ctx.module.id),
                    Some(right_ty_id),
                    ctx.infer,
                    ctx.types,
                );
                return Ok(*right);
            }

            let ty = self.infer_unary_operation(operator, &right_ty);
            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(right_ty_id),
                ctx.infer,
                ctx.types,
            );
            let ty_id = ctx.types.insert_type_from(ty, expression_id);
            self.apply_infer_state_type_freshness(ctx.types, ty_id, state);
            return Ok(ty_id);
        }

        // fall back to builtin inference when no operator interface exists
        let Some(operator_item) = operator_item else {
            let ty = self.infer_unary_operation(operator, &right_ty);
            let ty_id = ctx.types.insert_type_from(ty, expression_id);
            self.apply_infer_state_type_freshness(ctx.types, ty_id, state);
            return Ok(ty_id);
        };

        // require explicit operator interface implementation
        if !self.is_interface_implemented(ctx.symbol_type_view(), &right_ty, operator_item) {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                right_ty_id,
            );
            return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
        }

        // resolve the operator member function
        let operator_key = self.operator_member_key(operator_item);
        let Some(resolved) = ({
            self.resolve_member_function(
                &mut ctx.reborrow(),
                expression_id,
                right_id,
                Some(right_ty_id),
                &right_ty,
                &operator_key,
            )?
        }) else {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                right_ty_id,
            );
            return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                right_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                right_ty_id,
            );
            return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
        }

        // unary operators expect no dynamic parameters
        if !resolved.signature.parameters.is_empty() {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                right_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                right_ty_id,
            );
            return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(
            &mut ctx.reborrow(),
            expression_id,
            right_ty_id,
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

        Ok(return_ty_id)
    }

    /// Infer a binary operator expression.
    fn should_use_builtin_unary_operator(
        &self,
        operator: &UnaryOperator,
        right_ty: &Type,
        types: &TypeTable,
    ) -> bool {
        if operator.language_symbol().is_none() {
            return true;
        }

        if self.operator_operand_is_indeterminate(right_ty, types) {
            return true;
        }

        match operator {
            UnaryOperator::Negate | UnaryOperator::WrappingNegate | UnaryOperator::Plus => {
                self.is_numeric_like_type(right_ty, types)
            }
            UnaryOperator::ElementwiseNot => self.is_numeric_like_type(right_ty, types),
            UnaryOperator::Dereference => matches!(right_ty, Type::PointerOf { .. }),
            UnaryOperator::PostIncrement
            | UnaryOperator::PostDecrement
            | UnaryOperator::PreIncrement
            | UnaryOperator::PreDecrement
            | UnaryOperator::Not
            | UnaryOperator::Typeof
            | UnaryOperator::Void
            | UnaryOperator::Spread => true,
        }
    }
}
