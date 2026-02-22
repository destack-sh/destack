use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_unary_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        operator: &UnaryOperator,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_OPERATOR);

        let options = ctx.options;
        let mut right_ctx = ctx.fork().with_expected_type(None);
        let right_ty_id = self.infer_expression(
            module,
            right_id,
            tree,
            symbols,
            types,
            infer,
            &mut right_ctx,
        )?;
        let right_ty = types.get_type(right_ty_id).clone();

        let operator_item = operator.language_symbol();

        // use builtin rules when appropriate
        if self.should_use_builtin_unary_operator(operator, &right_ty, types) {
            if matches!(operator, UnaryOperator::Dereference)
                && let Type::PointerOf { right, .. } = &right_ty
            {
                self.commit_builtin_resolution(
                    expression_id.into_global_any(module.id),
                    Some(right_ty_id),
                    types,
                );
                return Ok(*right);
            }

            let ty = self.infer_unary_operation(operator, &right_ty);
            self.commit_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(right_ty_id),
                types,
            );
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // fall back to builtin inference when no operator interface exists
        let Some(operator_item) = operator_item else {
            let ty = self.infer_unary_operation(operator, &right_ty);
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // require explicit operator interface implementation
        if !self.is_interface_implemented(
            module,
            ctx.profile,
            &right_ty,
            operator_item,
            symbols,
            types,
        ) {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                right_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve the operator member function
        let operator_key = self.operator_member_key(operator_item);
        let Some(resolved) = self.resolve_member_function(
            module,
            expression_id,
            right_id,
            Some(right_ty_id),
            &right_ty,
            &operator_key,
            ctx.profile,
            &options,
            tree,
            symbols,
            types,
            infer,
        )?
        else {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                right_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.commit_member_call_resolution(
                module,
                ctx.profile,
                expression_id,
                right_ty_id,
                &resolved,
                tree,
                symbols,
                infer,
                types,
            )?;
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                right_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // unary operators expect no dynamic parameters
        if !resolved.signature.dynamic_parameters.is_empty() {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                right_ty_id,
                types,
            );
        }

        // finalize resolution and instance registration
        self.commit_member_call_resolution(
            module,
            ctx.profile,
            expression_id,
            right_ty_id,
            &resolved,
            tree,
            symbols,
            infer,
            types,
        )?;

        let return_ty_id = resolved.signature.return_type.unwrap_or_else(|| {
            types.insert_type_from(
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
