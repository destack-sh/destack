use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_binary_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        operator: &BinaryOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the left side first
        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

        // infer the right side after the left
        let right_ty_id =
            self.infer_expression(module, right_id, tree, symbols, types, infer, ctx)?;

        // load resolved operand types for operator checks
        let left_ty_id = self.unwrap_type_value(left_ty_id, types);
        let right_ty_id = self.unwrap_type_value(right_ty_id, types);
        let left_ty = types.get_type(left_ty_id).clone();
        let right_ty = types.get_type(right_ty_id).clone();
        let options = ctx.options;

        // resolve apparent operand types for builtin operator checks
        let left_operator_ty_id = self.normalize_apparent_type(
            module,
            ctx.profile,
            left_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::OPERATOR_COMPAT,
        );
        let right_operator_ty_id = self.normalize_apparent_type(
            module,
            ctx.profile,
            right_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::OPERATOR_COMPAT,
        );
        let left_operator_ty = types.get_type(left_operator_ty_id).clone();
        let right_operator_ty = types.get_type(right_operator_ty_id).clone();

        // enforce class-only instanceof targets
        if matches!(operator, BinaryOperator::InstanceOf)
            && matches!(module.source, ModuleSource::User)
        {
            let target_symbol =
                self.reference_symbol_for_expression(module, right_id, ctx.profile, tree, symbols);
            let is_class_target =
                target_symbol.is_some_and(|symbol| symbol.local_id.ty == SymbolType::Class);
            if !is_class_target {
                self.error(AnalyzeError::InvalidInstanceOfTarget {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // cache runtime check kind for instanceof guards
        if matches!(operator, BinaryOperator::InstanceOf) {
            let target_type_id = self.resolve_declared_type_expression(
                module,
                ctx.profile,
                right_id,
                tree,
                symbols,
                types,
                true,
                true,
            );
            if let Ok(target_type_id) = target_type_id {
                let target_type_id = self.unwrap_type_value(target_type_id, types);
                let runtime_check_kind = self.runtime_check_kind_for_relation(
                    module,
                    ctx.profile,
                    symbols,
                    left_ty_id,
                    target_type_id,
                    types,
                    &ctx.options,
                );
                if let Some(kind) = runtime_check_kind {
                    types.set_runtime_check_kind(expression_id.into_global_any(module.id), kind);
                }
            }
        }

        // track referential equality violations to avoid follow-up overload errors
        let mut referential_equality_violation = false;

        // reject referential equality when configured
        if options.no_referential_equality
            && matches!(module.source, ModuleSource::User)
            && matches!(
                operator,
                BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::EqualStrict
                    | BinaryOperator::NotEqualStrict
            )
        {
            let left_is_object =
                self.type_is_object_like(module, ctx.profile, left_ty_id, symbols, types);
            let right_is_object =
                self.type_is_object_like(module, ctx.profile, right_ty_id, symbols, types);
            if left_is_object || right_is_object {
                referential_equality_violation = true;
                self.error(AnalyzeError::ReferentialEqualityDisabled {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // short-circuit when referential equality is forbidden
        if referential_equality_violation {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            };
            return Ok(types.insert_type_from(ty, expression_id));
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
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    ty: struct_ty_id.into_global(module.id),
                });
            }
        }

        // handle coalesce operator separately
        if matches!(operator, BinaryOperator::Coalesce) {
            return self.infer_coalesce_expression(
                module,
                expression_id,
                left_id,
                right_id,
                left_ty_id,
                right_ty_id,
                &left_ty,
                &right_ty,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        // handle logical operators with operand unions
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            let result_ty_id = self.union_type(left_ty_id, right_ty_id, types);
            self.commit_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                types,
            );
            return Ok(result_ty_id);
        }

        // NOTE #Incomplete: full union equality depends on Equal overload dispatch
        // allow literal comparisons when values are assignable
        let is_literal_equality = self.should_use_literal_equality(
            operator,
            module,
            ctx.profile,
            symbols,
            left_ty_id,
            right_ty_id,
            left_id,
            right_id,
            &left_ty,
            &right_ty,
            tree,
            types,
            &options,
        );

        // use builtin rules when appropriate
        let operator_item = operator.language_symbol();
        if self.should_use_builtin_binary_operator(
            operator,
            &left_operator_ty,
            &right_operator_ty,
            types,
        ) || is_literal_equality
        {
            let ty =
                self.infer_binary_operation(operator, &left_operator_ty, &right_operator_ty, types);
            self.commit_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                types,
            );
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let Some(operator_item) = operator_item else {
            let ty = self.infer_binary_operation(operator, &left_ty, &right_ty, types);
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // require explicit operator interface implementation
        if !self.is_interface_implemented(
            module,
            ctx.profile,
            &left_ty,
            operator_item,
            symbols,
            types,
        ) {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                left_ty_id,
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
            left_id,
            Some(left_ty_id),
            &left_ty,
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
                left_ty_id,
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
                left_ty_id,
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
                left_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // binary operators expect one dynamic parameter
        let parameter_ty_id = resolved.signature.dynamic_parameters.first().copied();
        if resolved.signature.dynamic_parameters.len() != 1 {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                left_ty_id,
                types,
            );
        }

        // check argument assignability
        if let Some(parameter_ty_id) = parameter_ty_id {
            infer.push_constraint(Constraint::Subtype {
                sub_type: right_ty_id,
                super_type: parameter_ty_id,
                variance: None,
            });

            self.enforce_assignability_or_defer_unassignable_diagnostic(
                module,
                ctx.profile,
                expression_id.into_any(),
                parameter_ty_id,
                right_ty_id,
                symbols,
                types,
                infer,
                &options,
                UnassignableRelationFailureMode::PropagateError,
            )?;
        }

        // finalize resolution and instance registration
        self.commit_member_call_resolution(
            module,
            ctx.profile,
            expression_id,
            left_ty_id,
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
            return Ok(types.insert_type_from(boolean_ty, expression_id));
        }

        Ok(return_ty_id)
    }

    /// Infer an assignment expression.

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
            | BinaryOperator::In
            | BinaryOperator::InstanceOf => true,
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
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        left_ty: &Type,
        right_ty: &Type,
        tree: &NodeTree,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
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
            .literal_type_id_for_expression(left_id, tree, types)
            .or_else(|| self.is_literal_value_type(left_ty).then_some(left_ty_id));

        let right_literal_type_id = self
            .literal_type_id_for_expression(right_id, tree, types)
            .or_else(|| self.is_literal_value_type(right_ty).then_some(right_ty_id));

        // allow comparisons when the left literal is assignable
        if let Some(left_literal_type_id) = left_literal_type_id {
            let assignable = self.is_type_assignable(
                module,
                profile,
                symbols,
                right_ty_id,
                left_literal_type_id,
                types,
                options,
            );

            return assignable.is_assignable();
        }

        // allow comparisons when the right literal is assignable
        if let Some(right_literal_type_id) = right_literal_type_id {
            let assignable = self.is_type_assignable(
                module,
                profile,
                symbols,
                left_ty_id,
                right_literal_type_id,
                types,
                options,
            );

            return assignable.is_assignable();
        }

        false
    }
}
