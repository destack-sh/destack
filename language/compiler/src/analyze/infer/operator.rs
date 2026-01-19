use super::call::ResolvedMemberFunction;
use super::member::{MemberLookupMode, MemberResolution};
use super::{
    index_key_kind_for_index, index_key_kind_for_type, index_key_kinds_compatible_for_access,
};
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, AnalyzeWarning, Assignability, Compiler,
    InferContext, OperatorLanguageSymbolExt,
};
use destack_builtin::LanguageSymbol;
use destack_dir::{
    BinaryOperator, Constraint, DynamicKey, Expression, InferTable, LocalInstanceId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, NormalizationMode, PrimitiveType, ResolvedSignature,
    ScalarLiteral, StaticKey, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable, UnaryOperator,
};
use destack_workspace::{Module, ModuleSource, ProfileId};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a unary operator expression.
    pub(super) fn infer_unary_expression(
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
        let options = ctx.options;
        let right_ty_id =
            self.infer_expression(module, right_id, tree, symbols, types, infer, ctx)?;
        let right_ty = types.get_type(right_ty_id).clone();

        let operator_item = operator.language_symbol();

        // use builtin rules when appropriate
        if self.should_use_builtin_unary_operator(operator, &right_ty, types) {
            if matches!(operator, UnaryOperator::Dereference)
                && let Type::PointerOf { right, .. } = &right_ty
            {
                self.record_builtin_resolution(
                    expression_id.into_global_any(module.id),
                    Some(right_ty_id),
                    types,
                );
                return Ok(*right);
            }

            let ty = self.infer_unary_operation(operator, &right_ty);
            self.record_builtin_resolution(
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
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: right_ty_id.into_global(module.id),
            });
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
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                module,
                expression_id,
                right_ty_id,
                &resolved,
                types,
            );
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: right_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // unary operators expect no dynamic parameters
        if !resolved.signature.dynamic_parameters.is_empty() {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: right_ty_id.into_global(module.id),
            });
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(module, expression_id, right_ty_id, &resolved, types);

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
    pub(super) fn infer_binary_expression(
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
        let left_ty = types.get_type(left_ty_id).clone();
        let right_ty = types.get_type(right_ty_id).clone();
        let options = ctx.options;

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
            let left_is_object = self.type_is_object_like(left_ty_id, types);
            let right_is_object = self.type_is_object_like(right_ty_id, types);
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

        let operator_item = operator.language_symbol();

        // use builtin rules when appropriate
        if self.should_use_builtin_binary_operator(operator, &left_ty, &right_ty, types) {
            let ty = self.infer_binary_operation(operator, &left_ty, &right_ty, types);
            self.record_builtin_resolution(
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
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: left_ty_id.into_global(module.id),
            });
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
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(module, expression_id, left_ty_id, &resolved, types);
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // binary operators expect one dynamic parameter
        let parameter_ty_id = resolved.signature.dynamic_parameters.first().copied();
        if resolved.signature.dynamic_parameters.len() != 1 {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: left_ty_id.into_global(module.id),
            });
        }

        // check argument assignability
        if let Some(parameter_ty_id) = parameter_ty_id {
            infer.push_constraint(Constraint::Subtype {
                sub_type: right_ty_id,
                super_type: parameter_ty_id,
                variance: None,
            });

            if !self.is_infer_var_type(parameter_ty_id, types)
                && !self.is_infer_var_type(right_ty_id, types)
                && self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    parameter_ty_id,
                    right_ty_id,
                    types,
                    &options,
                ) == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    expected_ty: parameter_ty_id.into_global(module.id),
                    actual_ty: right_ty_id.into_global(module.id),
                });
            }
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(module, expression_id, left_ty_id, &resolved, types);

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

    /// Infer an assignment expression.
    pub(super) fn infer_assign_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let options = ctx.options;

        // route index assignment to index set resolution
        if let Expression::Index { left: _, right: _ } = tree.get(left_id) {
            return self.infer_index_assignment_expression(
                module,
                expression_id,
                left_id,
                right_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

        // use the left type as the expected type for the right expression
        let mut right_ctx = ctx.fork().with_expected_type(Some(left_ty_id));
        let right_ty_id = self.infer_expression(
            module,
            right_id,
            tree,
            symbols,
            types,
            infer,
            &mut right_ctx,
        )?;

        // enforce explicit ownership when implicit managed values are disabled
        self.check_no_implicit_managed_value(
            module,
            ctx.profile,
            right_id,
            left_ty_id,
            right_ty_id,
            tree,
            types,
            &options,
        );

        // add the subtype constraint
        infer.push_constraint(Constraint::Subtype {
            sub_type: right_ty_id,
            super_type: left_ty_id,
            variance: None,
        });

        // check assignability when types are resolved
        if !self.is_infer_var_type(left_ty_id, types)
            && !self.is_infer_var_type(right_ty_id, types)
            && self.is_type_assignable(
                module,
                ctx.profile,
                symbols,
                left_ty_id,
                right_ty_id,
                types,
                &options,
            ) == Assignability::NotAssignable
        {
            return Err(AnalyzeError::UnassignableType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                expected_ty: left_ty_id.into_global(module.id),
                actual_ty: right_ty_id.into_global(module.id),
            });
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a compound assignment expression.
    pub(super) fn infer_assign_binary_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let _right_ty_id =
            self.infer_expression(module, right_id, tree, symbols, types, infer, ctx)?;

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a coalesce expression.
    pub(super) fn infer_coalesce_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        _right_id: LocalNodeId<Expression>,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        _left_ty: &Type,
        _right_ty: &Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // strip nullish before Try evaluation
        let (non_nullish_ty_id, has_nullish) = self.strip_nullish_from_union(left_ty_id, types);
        let Some(non_nullish_ty_id) = non_nullish_ty_id else {
            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                types,
            );
            return Ok(right_ty_id);
        };

        // split Try and non-Try elements
        let (try_elements, non_try_elements) =
            self.split_try_elements(module, ctx.profile, non_nullish_ty_id, symbols, types);

        // no Try elements means nullish-only behavior
        if try_elements.is_empty() {
            let result_ty_id = if has_nullish {
                self.union_type(non_nullish_ty_id, right_ty_id, types)
            } else {
                left_ty_id
            };

            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                types,
            );

            return Ok(result_ty_id);
        }

        // collect Try success types for ??: remove nullish after unwrap
        let value_types = self.try_coalesce_success_types(
            module,
            expression_id,
            left_id,
            &try_elements,
            ctx.profile,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // record Try branch resolution when the receiver is Try-only
        if non_try_elements.is_empty() {
            let non_nullish_ty = types.get_type(non_nullish_ty_id).clone();
            let branch = self.resolve_try_branch_member(
                module,
                expression_id,
                left_id,
                non_nullish_ty_id,
                &non_nullish_ty,
                ctx.profile,
                &ctx.options,
                tree,
                symbols,
                types,
                infer,
            )?;

            if !branch.resolved.has_member {
                self.error(AnalyzeError::NoOverload {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    receiver_ty: non_nullish_ty_id.into_global(module.id),
                });
            } else {
                let payloads = self.resolve_try_branch_payloads(
                    module,
                    expression_id,
                    left_id.into_any(),
                    &non_nullish_ty,
                    &branch.resolved,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                    &ctx.options,
                )?;

                if payloads.is_none() {
                    self.error(AnalyzeError::InvalidTryBranch {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                } else {
                    self.record_try_branch_resolution(
                        module,
                        expression_id,
                        non_nullish_ty_id,
                        &branch,
                        types,
                    );
                }
            }
        }

        let mut result_types = Vec::new();

        if !value_types.is_empty() {
            let source_type_id = value_types[0];
            let unioned = self.union_type_from_list(value_types, source_type_id, types);
            result_types.push(unioned);
        }

        result_types.extend(non_try_elements);

        result_types.push(right_ty_id);

        let result_ty_id = match result_types.len() {
            0 => right_ty_id,
            1 => result_types[0],
            _ => {
                let source_type_id = result_types[0];
                self.union_type_from_list(result_types, source_type_id, types)
            }
        };

        self.record_builtin_resolution(
            expression_id.into_global_any(module.id),
            Some(left_ty_id),
            types,
        );

        Ok(result_ty_id)
    }

    /// Infer an index access expression.
    pub(super) fn infer_index_access_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        index_id: Option<LocalNodeId<Expression>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve receiver type
        let options = ctx.options;
        let receiver_ty_id =
            self.infer_expression(module, receiver_id, tree, symbols, types, infer, ctx)?;
        let receiver_ty = types.get_type(receiver_ty_id).clone();

        // short circuit index access on any
        if matches!(
            receiver_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            }
        ) {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Any,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // ensure instance types for reference receivers
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            receiver_ty_id,
            types,
        )?;

        // resolve index expression and literal string when possible
        let (index_ty_id, literal_string) = if let Some(index_id) = index_id {
            let index_ty_id =
                self.infer_expression(module, index_id, tree, symbols, types, infer, ctx)?;
            let literal_string = match tree.get(index_id) {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(name_id),
                } => Some(self.program.strings.get(*name_id)),
                _ => None,
            };
            (Some(index_ty_id), literal_string)
        } else {
            (None, None)
        };

        // reject computed property access when configured
        if options.no_computed_property_access
            && matches!(module.source, ModuleSource::User)
            && let Some(index_id) = index_id
        {
            let static_key = self.static_key_from_dynamic_key(
                ctx.profile,
                DynamicKey::Expression(index_id),
                tree,
                symbols,
                types,
            );
            if static_key.is_none() {
                self.error(AnalyzeError::ComputedPropertyAccessDisabled {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // handle builtin index access
        let builtin_ty_id = self.infer_builtin_index_access(
            &receiver_ty,
            receiver_ty_id,
            index_ty_id,
            literal_string.as_deref(),
            types,
            options.no_unchecked_indexed_access,
        );
        if let Some(builtin_ty_id) = builtin_ty_id {
            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                types,
            );
            return Ok(builtin_ty_id);
        }

        // guard non indexable receivers
        if !self.is_interface_implemented(
            module,
            ctx.profile,
            &receiver_ty,
            LanguageSymbol::Index,
            symbols,
            types,
        ) {
            self.error(AnalyzeError::NonIndexable {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve the index member function
        let member_key = self.operator_member_key(LanguageSymbol::Index);
        let Some(resolved) = self.resolve_member_function(
            module,
            expression_id,
            receiver_id,
            &receiver_ty,
            &member_key,
            ctx.profile,
            &options,
            tree,
            symbols,
            types,
            infer,
        )?
        else {
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                module,
                expression_id,
                receiver_ty_id,
                &resolved,
                types,
            );
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve index parameter type
        let parameter_ty_id = resolved.signature.dynamic_parameters.first().copied();
        if resolved.signature.dynamic_parameters.len() != 1 {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
        }

        // check index argument assignability
        if let (Some(parameter_ty_id), Some(index_ty_id)) = (parameter_ty_id, index_ty_id) {
            infer.push_constraint(Constraint::Subtype {
                sub_type: index_ty_id,
                super_type: parameter_ty_id,
                variance: None,
            });

            if !self.is_infer_var_type(parameter_ty_id, types)
                && !self.is_infer_var_type(index_ty_id, types)
                && self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    parameter_ty_id,
                    index_ty_id,
                    types,
                    &options,
                ) == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    expected_ty: parameter_ty_id.into_global(module.id),
                    actual_ty: index_ty_id.into_global(module.id),
                });
            }
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(module, expression_id, receiver_ty_id, &resolved, types);

        // resolve return type
        let value_ty_id = resolved.signature.return_type.unwrap_or_else(|| {
            types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            )
        });

        Ok(value_ty_id)
    }

    /// Infer an index assignment expression.
    pub(super) fn infer_index_assignment_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        index_expression_id: LocalNodeId<Expression>,
        value_expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // extract receiver and index expressions
        let Expression::Index {
            left: receiver_id,
            right: index_id,
        } = tree.get(index_expression_id)
        else {
            unreachable!("index assignment expects an index expression");
        };

        // resolve receiver type
        let options = ctx.options;
        let receiver_ty_id =
            self.infer_expression(module, *receiver_id, tree, symbols, types, infer, ctx)?;
        let receiver_ty = types.get_type(receiver_ty_id).clone();

        // resolve index expression and literal string when possible
        let (index_ty_id, literal_string) = if let Some(index_id) = index_id {
            let index_ty_id =
                self.infer_expression(module, *index_id, tree, symbols, types, infer, ctx)?;
            let literal_string = match tree.get(*index_id) {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(name_id),
                } => Some(self.program.strings.get(*name_id)),
                _ => None,
            };
            (Some(index_ty_id), literal_string)
        } else {
            (None, None)
        };

        // reject computed property access when configured
        if options.no_computed_property_access
            && matches!(module.source, ModuleSource::User)
            && let Some(index_id) = *index_id
        {
            let static_key = self.static_key_from_dynamic_key(
                ctx.profile,
                DynamicKey::Expression(index_id),
                tree,
                symbols,
                types,
            );
            if static_key.is_none() {
                self.error(AnalyzeError::ComputedPropertyAccessDisabled {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // handle builtin index assignment
        let builtin_value_ty_id = self.infer_builtin_index_access(
            &receiver_ty,
            receiver_ty_id,
            index_ty_id,
            literal_string.as_deref(),
            types,
            false,
        );
        if let Some(builtin_value_ty_id) = builtin_value_ty_id {
            let mut value_ctx = ctx.fork().with_expected_type(Some(builtin_value_ty_id));
            let value_ty_id = self.infer_expression(
                module,
                value_expression_id,
                tree,
                symbols,
                types,
                infer,
                &mut value_ctx,
            )?;

            // enforce explicit ownership when implicit managed values are disabled
            self.check_no_implicit_managed_value(
                module,
                ctx.profile,
                value_expression_id,
                builtin_value_ty_id,
                value_ty_id,
                tree,
                types,
                &options,
            );

            infer.push_constraint(Constraint::Subtype {
                sub_type: value_ty_id,
                super_type: builtin_value_ty_id,
                variance: None,
            });

            if !self.is_infer_var_type(builtin_value_ty_id, types)
                && !self.is_infer_var_type(value_ty_id, types)
                && self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    builtin_value_ty_id,
                    value_ty_id,
                    types,
                    &options,
                ) == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    expected_ty: builtin_value_ty_id.into_global(module.id),
                    actual_ty: value_ty_id.into_global(module.id),
                });
            }

            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                types,
            );

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // guard non indexable receivers
        if !self.is_interface_implemented(
            module,
            ctx.profile,
            &receiver_ty,
            LanguageSymbol::IndexSet,
            symbols,
            types,
        ) {
            self.error(AnalyzeError::NonIndexable {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve the index set member function
        let member_key = self.operator_member_key(LanguageSymbol::IndexSet);
        let Some(resolved) = self.resolve_member_function(
            module,
            expression_id,
            *receiver_id,
            &receiver_ty,
            &member_key,
            ctx.profile,
            &options,
            tree,
            symbols,
            types,
            infer,
        )?
        else {
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                module,
                expression_id,
                receiver_ty_id,
                &resolved,
                types,
            );
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve index set parameter types
        let key_param_ty_id = resolved.signature.dynamic_parameters.first().copied();
        let value_param_ty_id = resolved.signature.dynamic_parameters.get(1).copied();
        if resolved.signature.dynamic_parameters.len() != 2 {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
        }

        // check key argument assignability
        if let (Some(key_param_ty_id), Some(index_ty_id)) = (key_param_ty_id, index_ty_id) {
            infer.push_constraint(Constraint::Subtype {
                sub_type: index_ty_id,
                super_type: key_param_ty_id,
                variance: None,
            });
        }

        // infer value expression with contextual typing
        let mut value_ctx = ctx.fork().with_expected_type(value_param_ty_id);
        let value_ty_id = self.infer_expression(
            module,
            value_expression_id,
            tree,
            symbols,
            types,
            infer,
            &mut value_ctx,
        )?;

        // enforce explicit ownership when implicit managed values are disabled
        if let Some(value_param_ty_id) = value_param_ty_id {
            self.check_no_implicit_managed_value(
                module,
                ctx.profile,
                value_expression_id,
                value_param_ty_id,
                value_ty_id,
                tree,
                types,
                &options,
            );
        }

        // check value argument assignability
        if let Some(value_param_ty_id) = value_param_ty_id {
            infer.push_constraint(Constraint::Subtype {
                sub_type: value_ty_id,
                super_type: value_param_ty_id,
                variance: None,
            });

            if !self.is_infer_var_type(value_param_ty_id, types)
                && !self.is_infer_var_type(value_ty_id, types)
                && self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    value_param_ty_id,
                    value_ty_id,
                    types,
                    &options,
                ) == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    expected_ty: value_param_ty_id.into_global(module.id),
                    actual_ty: value_ty_id.into_global(module.id),
                });
            }
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(module, expression_id, receiver_ty_id, &resolved, types);

        // return void for index assignment
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a try unwrap expression.
    pub(super) fn infer_try_unwrap_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the receiver type
        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let left_ty = types.get_type(left_ty_id).clone();

        // handle Try unions by aggregating payload types
        if let Type::Union { elements } = &left_ty {
            let mut value_types = Vec::new();
            let mut error_types = Vec::new();

            // validate each Try union element and collect payload types
            for element_id in elements {
                // require Try on each union element
                let element_ty = types.get_type(*element_id).clone();
                let implements_try = self.is_interface_implemented(
                    module,
                    ctx.profile,
                    &element_ty,
                    LanguageSymbol::Try,
                    symbols,
                    types,
                );
                if !implements_try {
                    self.error(AnalyzeError::NoOverload {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        receiver_ty: left_ty_id.into_global(module.id),
                    });
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                }

                // resolve the branch member for the receiver
                let branch = self.resolve_try_branch_member(
                    module,
                    expression_id,
                    left_id,
                    *element_id,
                    &element_ty,
                    ctx.profile,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                if !branch.resolved.has_member {
                    self.error(AnalyzeError::NoOverload {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        receiver_ty: element_id.into_global(module.id),
                    });
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                }

                // resolve payload types for the receiver and branch
                let payloads = self.resolve_try_branch_payloads(
                    module,
                    expression_id,
                    left_id.into_any(),
                    &element_ty,
                    &branch.resolved,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                    &ctx.options,
                )?;
                let Some((value_ty_id, error_ty_id)) = payloads else {
                    self.error(AnalyzeError::InvalidTryBranch {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                    let ty = Type::Error;
                    return Ok(types.insert_type_from(ty, expression_id));
                };

                value_types.push(value_ty_id);
                error_types.push(error_ty_id);
            }

            // build union types for the merged branch payloads
            let fallback_type = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let value_source = value_types
                .first()
                .copied()
                .unwrap_or_else(|| types.insert_type_from(fallback_type.clone(), expression_id));
            let error_source = error_types
                .first()
                .copied()
                .unwrap_or_else(|| types.insert_type_from(fallback_type, expression_id));
            let value_ty_id = self.union_type_from_list(value_types, value_source, types);
            let error_ty_id = self.union_type_from_list(error_types, error_source, types);

            // register the error types for any enclosing catch
            let is_caught = ctx.record_try_error(error_ty_id);
            if !is_caught {
                self.ensure_try_from_error(
                    module,
                    expression_id,
                    left_ty_id,
                    &left_ty,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                )?;
            }

            // warn about non Error try error types
            self.warn_try_error_type(module, expression_id, error_ty_id, symbols, types, ctx);

            return Ok(value_ty_id);
        }

        // require a Try implementation for non union receivers
        if !self.is_interface_implemented(
            module,
            ctx.profile,
            &left_ty,
            LanguageSymbol::Try,
            symbols,
            types,
        ) {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve the branch member on the receiver
        let branch = self.resolve_try_branch_member(
            module,
            expression_id,
            left_id,
            left_ty_id,
            &left_ty,
            ctx.profile,
            &ctx.options,
            tree,
            symbols,
            types,
            infer,
        )?;

        // reject missing branch members
        if !branch.resolved.has_member {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve payload types for the receiver and branch
        let payloads = self.resolve_try_branch_payloads(
            module,
            expression_id,
            left_id.into_any(),
            &left_ty,
            &branch.resolved,
            ctx.profile,
            tree,
            symbols,
            types,
            &ctx.options,
        )?;
        let Some((value_ty_id, error_ty_id)) = payloads else {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::Error;
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // register errors for the nearest catch, if present
        let is_caught = ctx.record_try_error(error_ty_id);

        // enforce Try return compatibility and fromError when not handled by catch
        if !is_caught {
            self.ensure_try_from_error(
                module,
                expression_id,
                left_ty_id,
                &left_ty,
                ctx.profile,
                tree,
                symbols,
                types,
            )?;
            self.ensure_try_return_type_compatibility(
                module,
                expression_id,
                error_ty_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        // warn when Try error types are non Error
        self.warn_try_error_type(module, expression_id, error_ty_id, symbols, types, ctx);

        // record the branch resolution for later phases
        self.record_try_branch_resolution(module, expression_id, left_ty_id, &branch, types);

        Ok(value_ty_id)
    }

    /// Resolve Try payload types from receiver static arguments.
    fn try_payload_types_from_receiver(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeIdAny,
        receiver_ty: &Type,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // resolve static arguments for the receiver reference
        let inherited = self.resolve_inherited_static_arguments(
            module,
            profile,
            receiver_id,
            receiver_ty,
            options,
            tree,
            symbols,
            types,
        )?;

        // require at least two static arguments for Try payloads
        let Some(value_argument) = inherited.arguments.first() else {
            return Ok(None);
        };
        let Some(error_argument) = inherited.arguments.get(1) else {
            return Ok(None);
        };

        // convert the static arguments into payload types
        let value_ty_id = self.convert_static_argument_type(value_argument, receiver_id, types);
        let error_ty_id = self.convert_static_argument_type(error_argument, receiver_id, types);

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Resolve Try payload types from the branch return shape.
    fn try_payload_types_from_branch_return(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        return_ty_id: LocalTypeId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // evaluate unevaluated return types before inspecting
        if matches!(types.get_type(return_ty_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, return_ty_id, tree, symbols, types)?;
        }

        // skip when the branch type already errored
        if matches!(types.get_type(return_ty_id), Type::Error) {
            return Ok(None);
        }

        // ensure alias instance types are available for normalization
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            expression_id.into_any(),
            return_ty_id,
            types,
        )?;

        // normalize alias references for structural inspection
        let normalized_id = self.normalize_type(
            module,
            profile,
            return_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
        );
        let normalized = types.get_type(normalized_id).clone();

        // resolve type alias references into their structural targets
        let elements = match normalized {
            Type::Union { elements } => elements,
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                let try_branch_symbol = self.language_symbol(LanguageSymbol::TryBranch);
                if symbol == try_branch_symbol {
                    let arguments = static_arguments.as_deref().unwrap_or(&[]);
                    let Some(value_argument) = arguments.first() else {
                        return Ok(None);
                    };
                    let Some(error_argument) = arguments.get(1) else {
                        return Ok(None);
                    };

                    let value_ty_id = self.convert_static_argument_type(
                        value_argument,
                        expression_id.into_any(),
                        types,
                    );
                    let error_ty_id = self.convert_static_argument_type(
                        error_argument,
                        expression_id.into_any(),
                        types,
                    );
                    return Ok(Some((value_ty_id, error_ty_id)));
                }

                if symbol.ty() != SymbolType::TypeAlias {
                    return Ok(None);
                }

                let Some(alias_target_id) = types.get_alias_target_type_id(symbol) else {
                    return Ok(None);
                };

                let options = self.analyze_context_options_for_module(module.id);
                let resolved_arguments = self.resolve_type_reference_static_arguments(
                    module,
                    profile,
                    expression_id.into_any(),
                    symbol,
                    static_arguments.as_deref(),
                    true,
                    &options,
                    tree,
                    symbols,
                    types,
                )?;
                let arguments = resolved_arguments
                    .as_deref()
                    .or(static_arguments.as_deref())
                    .unwrap_or(&[]);

                let alias_ty_id = if arguments.is_empty() {
                    alias_target_id
                } else {
                    let substitutions = self.build_type_parameter_substitutions_for_symbol(
                        module,
                        profile,
                        symbol,
                        expression_id.into_any(),
                        arguments,
                        tree,
                        symbols,
                        types,
                    );
                    if substitutions.is_empty() {
                        alias_target_id
                    } else {
                        let mut cache = HashMap::new();
                        self.substitute_static_parameters(
                            alias_target_id,
                            &substitutions,
                            types,
                            &mut cache,
                        )
                    }
                };

                if matches!(types.get_type(alias_ty_id), Type::Unevaluated(_)) {
                    self.evaluate_type(module, profile, alias_ty_id, tree, symbols, types)?;
                }

                let alias_type = types.get_type(alias_ty_id).clone();
                let Type::Union { elements } = alias_type else {
                    return Ok(None);
                };
                elements
            }
            _ => return Ok(None),
        };

        // prepare static keys for branch inspection
        let kind_key = StaticKey::Name(self.program.strings.intern("kind"));
        let value_key = StaticKey::Name(self.program.strings.intern("value"));
        let error_key = StaticKey::Name(self.program.strings.intern("error"));
        let ok_id = self.program.strings.intern("ok");
        let err_id = self.program.strings.intern("err");

        let mut ok_types = Vec::new();
        let mut err_types = Vec::new();

        for element_id in elements {
            let Type::Object { fields, .. } = types.get_type(element_id) else {
                return Ok(None);
            };

            let Some(kind_field) = fields.iter().find(|field| field.key == kind_key) else {
                return Ok(None);
            };
            if kind_field.is_optional {
                return Ok(None);
            }

            let kind_literal = match types.get_type(kind_field.ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(value)),
                } => *value,
                _ => return Ok(None),
            };

            if kind_literal == ok_id {
                let Some(value_field) = fields.iter().find(|field| field.key == value_key) else {
                    return Ok(None);
                };
                if value_field.is_optional {
                    return Ok(None);
                }
                ok_types.push(value_field.ty);
                continue;
            }

            if kind_literal == err_id {
                let Some(error_field) = fields.iter().find(|field| field.key == error_key) else {
                    return Ok(None);
                };
                if error_field.is_optional {
                    return Ok(None);
                }
                err_types.push(error_field.ty);
                continue;
            }

            return Ok(None);
        }

        if ok_types.is_empty() || err_types.is_empty() {
            return Ok(None);
        }

        let value_source = ok_types[0];
        let error_source = err_types[0];
        let value_ty_id = self.union_type_from_list(ok_types, value_source, types);
        let error_ty_id = self.union_type_from_list(err_types, error_source, types);

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Resolve Try payloads while validating branch compatibility.
    fn resolve_try_branch_payloads(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeIdAny,
        receiver_ty: &Type,
        branch: &ResolvedMemberFunction,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // resolve payloads from receiver static arguments when available
        let receiver_payloads = self.try_payload_types_from_receiver(
            module,
            profile,
            receiver_id,
            receiver_ty,
            options,
            tree,
            symbols,
            types,
        )?;

        // resolve payloads from the branch return shape
        let branch_payloads = if let Some(return_ty_id) = branch.signature.return_type {
            self.try_payload_types_from_branch_return(
                module,
                expression_id,
                return_ty_id,
                profile,
                tree,
                symbols,
                types,
            )?
        } else {
            None
        };

        // prefer receiver payloads when they are concrete
        let mut payloads = receiver_payloads;
        if let Some(receiver_payloads) = receiver_payloads
            && self.payloads_need_branch(receiver_payloads, types)
        {
            payloads = branch_payloads;
        } else if payloads.is_none() {
            payloads = branch_payloads;
        }

        let Some((value_ty_id, error_ty_id)) = payloads else {
            return Ok(None);
        };

        // validate branch return types when the receiver payloads are authoritative
        if let Some(receiver_payloads) = receiver_payloads
            && !self.payloads_need_branch(receiver_payloads, types)
        {
            let is_valid = self.validate_try_branch_return_type(
                module,
                expression_id,
                branch,
                receiver_payloads.0,
                receiver_payloads.1,
                profile,
                tree,
                symbols,
                types,
                options,
            )?;
            if !is_valid {
                return Ok(None);
            }
        }

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Decide whether receiver payloads should be replaced by branch payloads.
    fn payloads_need_branch(
        &self,
        payloads: (LocalTypeId, LocalTypeId),
        types: &TypeTable,
    ) -> bool {
        // prefer branch payloads when receiver payloads are unknown
        [payloads.0, payloads.1].iter().any(|ty_id| {
            matches!(
                types.get_type(*ty_id),
                Type::InferVar { .. }
                    | Type::TypeLiteral {
                        value: TypeLiteral::Unknown | TypeLiteral::Any
                    }
                    | Type::Error
            )
        })
    }

    /// Decide whether a Try payload type should be validated for assignability.
    fn should_check_try_payload(&self, ty_id: LocalTypeId, types: &TypeTable) -> bool {
        !matches!(
            types.get_type(ty_id),
            Type::InferVar { .. }
                | Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any
                }
                | Type::Error
        )
    }

    /// Resolve the Try branch member for a receiver type.
    fn resolve_try_branch_member(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<TryBranchMember> {
        // resolve the branch member function
        let member_key = self.try_branch_member_key();
        let Some(resolved) = self.resolve_member_function(
            module,
            expression_id,
            receiver_expression_id,
            receiver_ty,
            &member_key,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?
        else {
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
            return Ok(TryBranchMember {
                resolved: ResolvedMemberFunction {
                    signature: ResolvedSignature {
                        dynamic_parameters: Vec::new(),
                        return_type: None,
                        static_arguments: Vec::new(),
                    },
                    member_resolution: MemberResolution::None,
                    member_symbol: None,
                    instance_arguments: Vec::new(),
                    has_member: false,
                },
                member_instance_id: None,
            });
        };

        // handle missing member
        if !resolved.has_member {
            return Ok(TryBranchMember {
                resolved,
                member_instance_id: None,
            });
        }

        // branch expects no dynamic parameters
        if !resolved.signature.dynamic_parameters.is_empty() {
            self.error(AnalyzeError::NoOverload {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
        }

        // register instance if needed
        let mut member_instance_id = None;
        if let Some(member_symbol) = resolved.member_symbol {
            let mut instance_arguments = resolved.instance_arguments.clone();
            instance_arguments.extend(resolved.signature.static_arguments.clone());

            if !instance_arguments.is_empty() {
                member_instance_id = Some(self.register_instance_for_node(
                    expression_id.into_global_any(module.id),
                    member_symbol,
                    instance_arguments,
                    types,
                ));
            }
        }

        Ok(TryBranchMember {
            resolved,
            member_instance_id,
        })
    }

    /// Validate the Try branch return type against receiver payloads.
    fn validate_try_branch_return_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        branch: &ResolvedMemberFunction,
        value_ty_id: LocalTypeId,
        error_ty_id: LocalTypeId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<bool> {
        // require an explicit return type on the branch member
        let Some(return_ty_id) = branch.signature.return_type else {
            return Ok(false);
        };

        // evaluate unevaluated return types before checking assignability
        if matches!(types.get_type(return_ty_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, return_ty_id, tree, symbols, types)?;
        }

        // skip additional diagnostics when the branch already errors
        if matches!(types.get_type(return_ty_id), Type::Error) {
            return Ok(true);
        }

        // extract payload types from the branch return shape
        let Some((branch_value_ty_id, branch_error_ty_id)) = self
            .try_payload_types_from_branch_return(
                module,
                expression_id,
                return_ty_id,
                profile,
                tree,
                symbols,
                types,
            )?
        else {
            return Ok(false);
        };

        // compare payloads to the expected Try arguments when they are concrete
        if self.should_check_try_payload(value_ty_id, types)
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                value_ty_id,
                branch_value_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            return Ok(false);
        }

        if self.should_check_try_payload(error_ty_id, types)
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                error_ty_id,
                branch_error_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Record resolution for a Try branch lookup.
    fn record_try_branch_resolution(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        branch: &TryBranchMember,
        types: &mut TypeTable,
    ) {
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &branch.resolved.member_resolution,
            branch.member_instance_id,
            None,
            branch.resolved.has_member,
            types,
        );
    }

    /// Ensure a Try receiver exposes a static fromError constructor.
    fn ensure_try_from_error(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<()> {
        // resolve the fromError key once
        let from_error_key = StaticKey::Name(self.program.strings.intern("fromError"));

        // scan receiver variants for missing static members
        let mut missing_from_error = false;
        match receiver_ty {
            Type::Reference { symbol, .. } => {
                let mut visited = Vec::new();
                let member = self.resolve_member_symbol_for_symbol(
                    module,
                    *symbol,
                    &from_error_key,
                    MemberLookupMode::Value,
                    profile,
                    tree,
                    symbols,
                    types,
                    &mut visited,
                )?;
                if member.is_none() {
                    missing_from_error = true;
                }
            }
            Type::Union { elements } => {
                for element_id in elements {
                    let element_ty = types.get_type(*element_id);
                    if let Type::Reference { symbol, .. } = element_ty {
                        let mut visited = Vec::new();
                        let member = self.resolve_member_symbol_for_symbol(
                            module,
                            *symbol,
                            &from_error_key,
                            MemberLookupMode::Value,
                            profile,
                            tree,
                            symbols,
                            types,
                            &mut visited,
                        )?;
                        if member.is_none() {
                            missing_from_error = true;
                        }
                    }
                }
            }
            _ => {}
        }

        // report missing fromError implementations
        if missing_from_error {
            self.error(AnalyzeError::MissingMember {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                receiver_ty: receiver_ty_id.into_global(module.id),
                member_key: from_error_key,
            });
        }

        Ok(())
    }

    /// Ensure the enclosing function return type can accept a Try error.
    fn ensure_try_return_type_compatibility(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        error_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) {
        // require a return type in the current function
        let Some(return_ty_id) = ctx.return_type else {
            self.error(AnalyzeError::MissingTryReturnType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return;
        };

        // require a Try return type
        let return_ty = types.get_type(return_ty_id).clone();
        if !self.is_interface_implemented(
            module,
            ctx.profile,
            &return_ty,
            LanguageSymbol::Try,
            symbols,
            types,
        ) {
            self.error(AnalyzeError::MissingTryReturnType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return;
        }

        // resolve payload types from the return type
        let payloads = match self.try_payload_types_from_receiver(
            module,
            ctx.profile,
            expression_id.into_any(),
            &return_ty,
            &ctx.options,
            tree,
            symbols,
            types,
        ) {
            Ok(payloads) => payloads,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        let Some((value_ty_id, return_error_ty_id)) = payloads else {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return;
        };

        // resolve the return type branch signature
        let branch = match self.resolve_try_branch_member(
            module,
            expression_id,
            expression_id,
            return_ty_id,
            &return_ty,
            ctx.profile,
            &ctx.options,
            tree,
            symbols,
            types,
            infer,
        ) {
            Ok(branch) => branch,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        // reject missing branch members on the return type
        if !branch.resolved.has_member {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return;
        }

        // validate the branch return type against the return payload types
        let is_valid = match self.validate_try_branch_return_type(
            module,
            expression_id,
            &branch.resolved,
            value_ty_id,
            return_error_ty_id,
            ctx.profile,
            tree,
            symbols,
            types,
            &ctx.options,
        ) {
            Ok(is_valid) => is_valid,
            Err(error) => {
                self.error(error);
                return;
            }
        };
        if !is_valid {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return;
        }

        // relate propagated error to the return error type
        infer.push_constraint(Constraint::Subtype {
            sub_type: error_ty_id,
            super_type: return_error_ty_id,
            variance: None,
        });

        // report incompatible error types when inference is resolved
        if !self.is_infer_var_type(return_error_ty_id, types)
            && !self.is_infer_var_type(error_ty_id, types)
            && self.is_type_assignable(
                module,
                ctx.profile,
                symbols,
                return_error_ty_id,
                error_ty_id,
                types,
                &ctx.options,
            ) == Assignability::NotAssignable
        {
            self.error(AnalyzeError::UnassignableType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                expected_ty: return_error_ty_id.into_global(module.id),
                actual_ty: error_ty_id.into_global(module.id),
            });
        }
    }

    /// Warn when Try error types do not implement Error.
    fn warn_try_error_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        error_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
    ) {
        // only lint user modules
        if !matches!(module.source, ModuleSource::User) {
            return;
        }

        // skip unknown or inference-driven error types
        let error_ty = types.get_type(error_ty_id);
        if matches!(
            error_ty,
            Type::InferVar { .. }
                | Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any
                }
                | Type::Error
        ) {
            return;
        }

        // resolve the Error interface reference
        let error_symbol = self.language_symbol(LanguageSymbol::Error);
        let error_reference_id = types.insert_type_from_any(
            Type::Reference {
                symbol: error_symbol,
                static_arguments: None,
            },
            expression_id.into_any(),
        );

        // warn when error type is not assignable to Error
        if self.is_type_assignable(
            module,
            ctx.profile,
            symbols,
            error_reference_id,
            error_ty_id,
            types,
            &ctx.options,
        ) == Assignability::NotAssignable
        {
            self.warning(AnalyzeWarning::TryErrorNotError {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                ty: error_ty_id.into_global(module.id),
            });
        }
    }

    /// Split a type into Try and non-Try elements.
    fn split_try_elements(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> (Vec<LocalTypeId>, Vec<LocalTypeId>) {
        // collect Try and non-Try elements
        let mut try_elements = Vec::new();
        let mut non_try_elements = Vec::new();

        // split based on interface implementation
        match types.get_type(ty_id) {
            Type::Union { elements } => {
                for element_id in elements {
                    let element_ty = types.get_type(*element_id);
                    if self.is_interface_implemented(
                        module,
                        profile,
                        element_ty,
                        LanguageSymbol::Try,
                        symbols,
                        types,
                    ) {
                        try_elements.push(*element_id);
                    } else {
                        non_try_elements.push(*element_id);
                    }
                }
            }
            ty => {
                if self.is_interface_implemented(
                    module,
                    profile,
                    ty,
                    LanguageSymbol::Try,
                    symbols,
                    types,
                ) {
                    try_elements.push(ty_id);
                } else {
                    non_try_elements.push(ty_id);
                }
            }
        }

        // return the split lists
        (try_elements, non_try_elements)
    }

    /// Collect the Try success types for coalesce.
    fn try_coalesce_success_types(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        element_ids: &[LocalTypeId],
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // collect the success value types
        let mut value_types = Vec::new();

        // resolve branch types for each Try element
        for element_id in element_ids {
            let element_ty = types.get_type(*element_id).clone();
            let branch = self.resolve_try_branch_member(
                module,
                expression_id,
                receiver_expression_id,
                *element_id,
                &element_ty,
                profile,
                &ctx.options,
                tree,
                symbols,
                types,
                infer,
            )?;

            // reject missing branches for Try elements
            if !branch.resolved.has_member {
                self.error(AnalyzeError::NoOverload {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    receiver_ty: element_id.into_global(module.id),
                });
                continue;
            }

            // resolve payload types for the receiver and branch
            let payloads = self.resolve_try_branch_payloads(
                module,
                expression_id,
                receiver_expression_id.into_any(),
                &element_ty,
                &branch.resolved,
                profile,
                tree,
                symbols,
                types,
                &ctx.options,
            )?;
            let Some((value_ty_id, _error_ty_id)) = payloads else {
                self.error(AnalyzeError::InvalidTryBranch {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
                continue;
            };

            // collect ok branch types and strip nullish
            let (non_nullish, _) = self.strip_nullish_from_union(value_ty_id, types);
            if let Some(non_nullish) = non_nullish {
                value_types.push(non_nullish);
            }
        }

        // return collected success types
        Ok(value_types)
    }

    /// Infer builtin index access for arrays, tuples, and index signatures.
    ///
    /// Array and tuple access always return the element type. The
    /// `include_undefined` flag only applies to index signatures.
    fn infer_builtin_index_access(
        &self,
        receiver_ty: &Type,
        receiver_ty_id: LocalTypeId,
        index_ty_id: Option<LocalTypeId>,
        literal_string: Option<&str>,
        types: &mut TypeTable,
        include_undefined: bool,
    ) -> Option<LocalTypeId> {
        let add_unchecked_undefined = |ty_id: LocalTypeId, types: &mut TypeTable| {
            if include_undefined {
                let undefined_ty_id = types.insert_type_from_type(
                    Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    },
                    receiver_ty_id,
                );
                self.union_type(ty_id, undefined_ty_id, types)
            } else {
                ty_id
            }
        };

        match receiver_ty {
            Type::Array { element } => {
                let element_ty_id = element.unwrap_or_else(|| {
                    types.insert_type_from_type(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        receiver_ty_id,
                    )
                });
                Some(element_ty_id)
            }
            Type::Tuple { elements } => {
                if elements.is_empty() {
                    return None;
                }

                // index into tuple by integer
                if let Some(index_ty_id) = index_ty_id
                    && let Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(index)),
                    } = types.get_type(index_ty_id)
                    && *index >= 0
                {
                    let index = *index as usize;
                    if index < elements.len() {
                        return Some(elements[index].ty);
                    }
                }

                let elements = elements.iter().map(|element| element.ty).collect();
                let union_ty_id = self.union_type_from_list(elements, receiver_ty_id, types);
                Some(union_ty_id)
            }
            Type::Object {
                fields,
                index_signatures,
                ..
            } => {
                let index_ty_id = index_ty_id?;

                if let Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(name_id)),
                } = types.get_type(index_ty_id)
                {
                    let key = StaticKey::Name(*name_id);
                    if let Some(field) = fields.iter().find(|field| field.key.matches(&key)) {
                        return Some(field.ty);
                    }
                }

                let signature_ty_id = self.infer_index_signature_access(
                    index_signatures,
                    index_ty_id,
                    literal_string,
                    types,
                )?;
                Some(add_unchecked_undefined(signature_ty_id, types))
            }
            Type::Reference { symbol, .. } => {
                let instance_ty_id = types.get_instance_type_id(*symbol)?;
                let instance_ty = types.get_type(instance_ty_id).clone();
                self.infer_builtin_index_access(
                    &instance_ty,
                    instance_ty_id,
                    index_ty_id,
                    literal_string,
                    types,
                    include_undefined,
                )
            }
            _ => None,
        }
    }

    fn infer_index_signature_access(
        &self,
        index_signatures: &[destack_dir::TypeIndexSignature],
        index_ty_id: LocalTypeId,
        literal_string: Option<&str>,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let key_kind =
            index_key_kind_for_index(index_ty_id, literal_string, types, &self.program.strings)?;
        let mut value_types = Vec::new();

        for signature in index_signatures {
            let signature_kind = index_key_kind_for_type(signature.key_type, types);
            if index_key_kinds_compatible_for_access(signature_kind, key_kind) {
                value_types.push(signature.value_type);
            }
        }

        match value_types.len() {
            0 => None,
            1 => Some(value_types[0]),
            _ => {
                let source_type_id = value_types[0];
                Some(self.union_type_from_list(value_types, source_type_id, types))
            }
        }
    }

    /// Build the member key for Try.branch.
    fn try_branch_member_key(&self) -> StaticKey {
        let name_id = self.program.strings.intern("branch");
        StaticKey::Name(name_id)
    }

    /// Build the member key for a language item operator interface.
    fn operator_member_key(&self, operator_item: LanguageSymbol) -> StaticKey {
        let export_name = operator_item.export_name();
        let mut chars = export_name.chars();
        let first_char = chars.next().unwrap_or_default();

        let mut member_name = String::new();
        member_name.push(first_char.to_ascii_lowercase());
        member_name.push_str(chars.as_str());

        let name_id = self.program.strings.intern(&member_name);
        StaticKey::Name(name_id)
    }

    /// Check whether to use builtin unary operator rules for a type.
    fn should_use_builtin_unary_operator(
        &self,
        operator: &UnaryOperator,
        right_ty: &Type,
        types: &TypeTable,
    ) -> bool {
        if operator.language_symbol().is_none() {
            return true;
        }

        if self.is_unresolved_operator_type(right_ty, types) {
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
            | UnaryOperator::Spread => true,
        }
    }

    /// Check whether to use builtin binary operator rules for a type pair.
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

        if self.is_unresolved_operator_type(left_ty, types)
            || self.is_unresolved_operator_type(right_ty, types)
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
                self.is_primitive_literal_type(left_ty, types)
                    && self.is_primitive_literal_type(right_ty, types)
            }
            BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Coalesce
            | BinaryOperator::In
            | BinaryOperator::InstanceOf => true,
        }
    }
}

/// Resolved information for a Try branch lookup.
#[derive(Debug)]
struct TryBranchMember {
    /// The resolved branch member signature.
    resolved: ResolvedMemberFunction,
    /// The instance used by the branch member.
    member_instance_id: Option<LocalInstanceId>,
}
