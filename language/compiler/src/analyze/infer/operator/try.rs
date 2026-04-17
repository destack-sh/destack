use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_coalesce_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        _right_id: LocalNodeId<Expression>,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        _left_ty: &Type,
        _right_ty: &Type,
        _state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // strip nullish before Try evaluation
        let (non_nullish_ty_id, has_nullish) = self.strip_nullish_from_union(left_ty_id, ctx.types);
        let Some(non_nullish_ty_id) = non_nullish_ty_id else {
            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(left_ty_id),
                ctx.infer,
                ctx.types,
            );
            return Ok(right_ty_id);
        };

        // split Try and non-Try elements
        let (try_elements, non_try_elements) = self.split_try_elements(ctx, non_nullish_ty_id);

        // no Try elements means nullish-only behavior
        if try_elements.is_empty() {
            let result_ty_id = if has_nullish {
                self.union_type_from_list(
                    vec![non_nullish_ty_id, right_ty_id],
                    non_nullish_ty_id,
                    ctx.types,
                )
            } else {
                left_ty_id
            };

            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(left_ty_id),
                ctx.infer,
                ctx.types,
            );

            return Ok(result_ty_id);
        }

        // collect Try success types for ??: remove nullish after unwrap
        let value_types = self.try_coalesce_success_types(
            &mut ctx.reborrow(),
            expression_id,
            left_id,
            &try_elements,
        )?;

        // record Try branch resolution when the receiver is Try-only
        if non_try_elements.is_empty() {
            let non_nullish_ty = ctx.types.get_type(non_nullish_ty_id).clone();
            let branch = self.resolve_try_branch_member(
                &mut ctx.reborrow(),
                expression_id,
                left_id,
                non_nullish_ty_id,
                &non_nullish_ty,
            )?;

            if !branch.resolved.has_member {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    non_nullish_ty_id,
                );
            } else {
                let value_and_error_types = self.resolve_try_value_and_error_types(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_id.into_any(),
                    non_nullish_ty_id,
                    &non_nullish_ty,
                    &branch.resolved,
                )?;

                if value_and_error_types.is_none() {
                    self.error(AnalyzeError::InvalidTryBranch {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                } else {
                    self.record_try_branch_resolution(
                        &mut ctx.reborrow(),
                        expression_id,
                        non_nullish_ty_id,
                        &branch,
                    );
                }
            }
        }

        let mut result_types = Vec::new();

        if !value_types.is_empty() {
            let source_type_id = value_types[0];
            let unioned = self.union_type_from_list(value_types, source_type_id, ctx.types);
            result_types.push(unioned);
        }

        result_types.extend(non_try_elements);

        result_types.push(right_ty_id);

        let result_ty_id = match result_types.len() {
            0 => right_ty_id,
            1 => result_types[0],
            _ => {
                let source_type_id = result_types[0];
                self.union_type_from_list(result_types, source_type_id, ctx.types)
            }
        };

        self.record_provisional_builtin_resolution(
            expression_id.into_global_any(ctx.module.id),
            Some(left_ty_id),
            ctx.infer,
            ctx.types,
        );

        Ok(result_ty_id)
    }

    /// Infer an index access expression.
    pub(crate) fn infer_try_unwrap_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the receiver type
        let left_ty_id = self.infer_expression(&mut ctx.reborrow(), left_id, state)?;
        let left_ty = ctx.types.get_type(left_ty_id).clone();

        // handle Try unions by aggregating value and error types
        if let Type::Union { elements } = &left_ty {
            let mut value_types = Vec::new();
            let mut error_types = Vec::new();

            // validate each Try union element and collect value and error types
            for element_id in elements {
                // require Try on each union element
                let element_ty = ctx.types.get_type(*element_id).clone();
                let implements_try = self.is_interface_implemented(
                    ctx.symbol_type_view(),
                    &element_ty,
                    LanguageSymbol::Try,
                );
                if !implements_try {
                    self.emit_no_overload_for_receiver_type(
                        ctx.module_type_view(),
                        expression_id.into_any(),
                        left_ty_id,
                    );
                    let ty = Type::Error;
                    return Ok(ctx.types.insert_type_from(ty, expression_id));
                }

                // resolve the branch member for the receiver
                let branch = self.resolve_try_branch_member(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_id,
                    *element_id,
                    &element_ty,
                )?;
                if !branch.resolved.has_member {
                    self.emit_no_overload_for_receiver_type(
                        ctx.module_type_view(),
                        expression_id.into_any(),
                        *element_id,
                    );
                    let ty = Type::Error;
                    return Ok(ctx.types.insert_type_from(ty, expression_id));
                }

                // resolve value and error types for the receiver and branch
                let value_and_error_types = self.resolve_try_value_and_error_types(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_id.into_any(),
                    *element_id,
                    &element_ty,
                    &branch.resolved,
                )?;
                let Some((value_ty_id, error_ty_id)) = value_and_error_types else {
                    self.error(AnalyzeError::InvalidTryBranch {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                    let ty = Type::Error;
                    return Ok(ctx.types.insert_type_from(ty, expression_id));
                };

                value_types.push(value_ty_id);
                error_types.push(error_ty_id);
            }

            // build union types for merged branch value and error types
            let unknown_placeholder_type = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let value_source = value_types.first().copied().unwrap_or_else(|| {
                ctx.types
                    .insert_type_from(unknown_placeholder_type.clone(), expression_id)
            });
            let error_source = error_types.first().copied().unwrap_or_else(|| {
                ctx.types
                    .insert_type_from(unknown_placeholder_type, expression_id)
            });
            let value_ty_id = self.union_type_from_list(value_types, value_source, ctx.types);
            let error_ty_id = self.union_type_from_list(error_types, error_source, ctx.types);

            // register the error types for any enclosing catch
            let is_caught = state.record_try_error(error_ty_id);
            if !is_caught {
                self.ensure_try_from_error(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_ty_id,
                    &left_ty,
                )?;
            }

            // warn about non Error try error types
            self.warn_try_error_type(&mut ctx.reborrow(), expression_id, error_ty_id);

            return Ok(value_ty_id);
        }

        // require a Try implementation for non union receivers
        if !self.is_interface_implemented(ctx.symbol_type_view(), &left_ty, LanguageSymbol::Try) {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                left_ty_id,
            );
            let ty = Type::Error;
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // resolve the branch member on the receiver
        let branch = self.resolve_try_branch_member(
            &mut ctx.reborrow(),
            expression_id,
            left_id,
            left_ty_id,
            &left_ty,
        )?;

        // reject missing branch members
        if !branch.resolved.has_member {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                left_ty_id,
            );
            let ty = Type::Error;
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // resolve value and error types for the receiver and branch
        let value_and_error_types = self.resolve_try_value_and_error_types(
            &mut ctx.reborrow(),
            expression_id,
            left_id.into_any(),
            left_ty_id,
            &left_ty,
            &branch.resolved,
        )?;
        let Some((value_ty_id, error_ty_id)) = value_and_error_types else {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            let ty = Type::Error;
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        };

        // register errors for the nearest catch, if present
        let is_caught = state.record_try_error(error_ty_id);

        // enforce Try return compatibility and fromError when not handled by catch
        if !is_caught {
            self.ensure_try_from_error(&mut ctx.reborrow(), expression_id, left_ty_id, &left_ty)?;
            self.ensure_try_return_type_assignable(
                &mut ctx.reborrow(),
                expression_id,
                error_ty_id,
                state,
            )?;
        }

        // warn when Try error types are non Error
        self.warn_try_error_type(&mut ctx.reborrow(), expression_id, error_ty_id);

        // record the branch resolution for later phases
        self.record_try_branch_resolution(&mut ctx.reborrow(), expression_id, left_ty_id, &branch);

        Ok(value_ty_id)
    }

    /// Resolve Try value and error types from receiver static arguments.
    fn try_value_and_error_types_from_receiver(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // resolve static arguments for the receiver reference
        let inherited = self.resolve_inherited_static_arguments(
            &mut ctx.reborrow(),
            receiver_id,
            Some(receiver_ty_id),
            receiver_ty,
        )?;

        // require at least two static arguments for Try value and error types
        let Some(value_argument) = inherited.arguments.first() else {
            return Ok(None);
        };
        let Some(error_argument) = inherited.arguments.get(1) else {
            return Ok(None);
        };

        // convert static arguments into value and error types
        let value_ty_id = self.convert_static_argument_type(value_argument, receiver_id, ctx.types);
        let error_ty_id = self.convert_static_argument_type(error_argument, receiver_id, ctx.types);

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Resolve Try value and error types from the branch return shape.
    fn try_branch_value_and_error_types_from_return(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        return_ty_id: LocalTypeId,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // evaluate unevaluated return types before inspecting
        if matches!(ctx.types.get_type(return_ty_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(&mut ctx.type_context_reborrow(), return_ty_id)?;
        }

        // skip when the branch type already errored
        if matches!(ctx.types.get_type(return_ty_id), Type::Error) {
            return Ok(None);
        }

        // ensure alias instance types are available for normalization
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            return_ty_id,
        )?;

        // normalize alias references for structural inspection
        let normalized_id = self.normalize_type(
            &mut ctx.type_context_reborrow(),
            return_ty_id,
            NormalizationMode::Assign,
        );
        let normalized = ctx.types.get_type(normalized_id).clone();

        // resolve type alias references into their structural targets
        let elements = match normalized {
            Type::Union { elements } => elements,
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let try_branch_symbol =
                    self.language_symbol(ctx.profile, LanguageSymbol::TryBranch);
                if canonical_symbol == try_branch_symbol {
                    let resolved_arguments = self.resolve_type_reference_static_arguments(
                        &mut ctx.type_context_reborrow(),
                        expression_id.into_any(),
                        canonical_symbol,
                        static_arguments.as_deref(),
                        true,
                    )?;
                    let arguments = resolved_arguments
                        .as_deref()
                        .or(static_arguments.as_deref())
                        .unwrap_or(&[]);
                    let Some(value_argument) = arguments.first() else {
                        return Ok(None);
                    };
                    let Some(error_argument) = arguments.get(1) else {
                        return Ok(None);
                    };

                    let value_ty_id = self.convert_static_argument_type(
                        value_argument,
                        expression_id.into_any(),
                        ctx.types,
                    );
                    let error_ty_id = self.convert_static_argument_type(
                        error_argument,
                        expression_id.into_any(),
                        ctx.types,
                    );
                    return Ok(Some((value_ty_id, error_ty_id)));
                }

                if canonical_symbol.ty() != SymbolType::TypeAlias {
                    return Ok(None);
                }

                let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
                    &mut ctx.type_context_reborrow(),
                    canonical_symbol,
                    expression_id.into_any(),
                ) else {
                    return Ok(None);
                };

                let resolved_arguments = self.resolve_type_reference_static_arguments(
                    &mut ctx.type_context_reborrow(),
                    expression_id.into_any(),
                    canonical_symbol,
                    static_arguments.as_deref(),
                    true,
                )?;
                let arguments = resolved_arguments
                    .as_deref()
                    .or(static_arguments.as_deref())
                    .unwrap_or(&[]);

                let alias_ty_id = if arguments.is_empty() {
                    alias_target_id
                } else {
                    let substitutions = self.build_type_parameter_substitutions_for_symbol(
                        &mut ctx.type_context_reborrow(),
                        canonical_symbol,
                        expression_id.into_any(),
                        arguments,
                    );
                    if substitutions.is_empty() {
                        alias_target_id
                    } else {
                        let mut cache = HashMap::new();
                        self.substitute_static_parameters(
                            alias_target_id,
                            &substitutions,
                            ctx.types,
                            &mut cache,
                        )
                    }
                };

                if matches!(ctx.types.get_type(alias_ty_id), Type::Unevaluated(_)) {
                    self.resolve_declared_type(&mut ctx.type_context_reborrow(), alias_ty_id)?;
                }

                let alias_type = ctx.types.get_type(alias_ty_id).clone();
                let Type::Union { elements } = alias_type else {
                    return Ok(None);
                };
                elements
            }
            _ => return Ok(None),
        };

        // prepare static keys for branch inspection
        let kind_key = StaticKey::Name(self.repository.strings.intern("kind"));
        let value_key = StaticKey::Name(self.repository.strings.intern("value"));
        let error_key = StaticKey::Name(self.repository.strings.intern("error"));
        let ok_id = self.repository.strings.intern("ok");
        let err_id = self.repository.strings.intern("err");

        // collect ok/error value types from union members
        let mut ok_types = Vec::new();
        let mut err_types = Vec::new();

        for element_id in elements {
            let Type::Object { fields, .. } = ctx.types.get_type(element_id) else {
                return Ok(None);
            };

            let Some(kind_field) = fields.iter().find(|field| field.key == kind_key) else {
                return Ok(None);
            };
            if kind_field.is_optional {
                return Ok(None);
            }

            let kind_literal = match ctx.types.get_type(kind_field.ty) {
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
        let value_ty_id = self.union_type_from_list(ok_types, value_source, ctx.types);
        let error_ty_id = self.union_type_from_list(err_types, error_source, ctx.types);

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Resolve Try value and error types while validating branch assignability.
    fn resolve_try_value_and_error_types(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        branch: &ResolvedMemberFunction,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // resolve value and error types from receiver static arguments when available
        let receiver_value_and_error_types = self.try_value_and_error_types_from_receiver(
            &mut ctx.reborrow(),
            receiver_id,
            receiver_ty_id,
            receiver_ty,
        )?;

        // resolve value and error types from the branch return shape
        let branch_value_and_error_types = if let Some(return_ty_id) = branch.signature.return_type
        {
            self.try_branch_value_and_error_types_from_return(
                &mut ctx.reborrow(),
                expression_id,
                return_ty_id,
            )?
        } else {
            None
        };

        // prefer receiver value and error types when they are concrete
        let mut value_and_error_types = receiver_value_and_error_types;
        if let Some(receiver_value_and_error_types) = receiver_value_and_error_types
            && self.try_value_and_error_types_need_branch(receiver_value_and_error_types, ctx.types)
        {
            value_and_error_types = branch_value_and_error_types;
        } else if value_and_error_types.is_none() {
            value_and_error_types = branch_value_and_error_types;
        }

        let Some((value_ty_id, error_ty_id)) = value_and_error_types else {
            return Ok(None);
        };

        // validate branch return types when receiver value and error types are authoritative
        if let Some(receiver_value_and_error_types) = receiver_value_and_error_types
            && !self
                .try_value_and_error_types_need_branch(receiver_value_and_error_types, ctx.types)
        {
            let is_valid = self.check_try_branch_return_type_assignable(
                &mut ctx.reborrow(),
                expression_id,
                branch,
                receiver_value_and_error_types.0,
                receiver_value_and_error_types.1,
            )?;
            if !is_valid {
                return Ok(None);
            }
        }

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Decide whether receiver value and error types should be replaced by branch types.
    fn try_value_and_error_types_need_branch(
        &self,
        value_and_error_types: (LocalTypeId, LocalTypeId),
        types: &TypeTable,
    ) -> bool {
        // prefer branch types when receiver value and error types are unknown
        [value_and_error_types.0, value_and_error_types.1]
            .iter()
            .any(|ty_id| {
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

    /// Decide whether one Try value or error type should be validated for assignability.
    fn should_check_try_value_error_assignability(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
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
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
    ) -> AnalyzeResult<TryBranchMember> {
        // resolve the branch member function
        let member_key = self.try_branch_member_key();
        let Some(resolved) = self.resolve_member_function(
            &mut ctx.reborrow(),
            expression_id,
            receiver_expression_id,
            Some(receiver_ty_id),
            receiver_ty,
            &member_key,
        )?
        else {
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
                    signature_static_parameter_symbols: Vec::new(),
                    bound_substitutions: HashMap::new(),
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
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
        }

        // register instance if needed
        let member_instance_id =
            self.record_member_call_instance_id(&mut ctx.reborrow(), expression_id, &resolved)?;

        Ok(TryBranchMember {
            resolved,
            member_instance_id,
        })
    }

    /// Validate the Try branch return type against receiver value and error types.
    fn check_try_branch_return_type_assignable(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        branch: &ResolvedMemberFunction,
        value_ty_id: LocalTypeId,
        error_ty_id: LocalTypeId,
    ) -> AnalyzeResult<bool> {
        // require an explicit return type on the branch member
        let Some(return_ty_id) = branch.signature.return_type else {
            return Ok(false);
        };

        // evaluate unevaluated return types before checking assignability
        if matches!(ctx.types.get_type(return_ty_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(&mut ctx.type_context_reborrow(), return_ty_id)?;
        }

        // skip additional diagnostics when the branch already errors
        if matches!(ctx.types.get_type(return_ty_id), Type::Error) {
            return Ok(true);
        }

        // extract value and error types from the branch return shape
        let Some((branch_value_ty_id, branch_error_ty_id)) = self
            .try_branch_value_and_error_types_from_return(
                &mut ctx.reborrow(),
                expression_id,
                return_ty_id,
            )?
        else {
            let is_try_branch = matches!(ctx.types.get_type(return_ty_id), Type::Reference { symbol, .. } if {
                let canonical = self.canonical_symbol_id(ctx.module_symbol_view(), *symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                canonical == self.language_symbol(ctx.profile, LanguageSymbol::TryBranch)
            });
            return Ok(is_try_branch);
        };

        // compare value and error types to the expected Try arguments when they are concrete
        let branch_value_is_parameter = matches!(
            ctx.types.get_type(branch_value_ty_id),
            Type::Reference { symbol, .. }
                if self.symbol_is_static_parameter(
                ctx.symbol_type_view(),
                *symbol,
            )
        );
        let branch_error_is_parameter = matches!(
            ctx.types.get_type(branch_error_ty_id),
            Type::Reference { symbol, .. }
                if self.symbol_is_static_parameter(
                ctx.symbol_type_view(),
                *symbol,
            )
        );

        if self.should_check_try_value_error_assignability(value_ty_id, ctx.types)
            && !branch_value_is_parameter
            && self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                value_ty_id,
                branch_value_ty_id,
            ) == Assignability::NotAssignable
        {
            return Ok(false);
        }

        if self.should_check_try_value_error_assignability(error_ty_id, ctx.types)
            && !branch_error_is_parameter
            && self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                error_ty_id,
                branch_error_ty_id,
            ) == Assignability::NotAssignable
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Record resolution for a Try branch lookup.
    fn record_try_branch_resolution(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        branch: &TryBranchMember,
    ) {
        self.record_provisional_member_resolution(
            expression_id.into_global_any(ctx.module.id),
            Some(receiver_ty_id),
            &branch.resolved.member_resolution,
            branch.member_instance_id,
            None,
            branch.resolved.has_member,
            ctx.infer,
            ctx.types,
        );
    }

    /// Ensure a Try receiver exposes a static fromError constructor.
    fn ensure_try_from_error(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
    ) -> AnalyzeResult<()> {
        // resolve the fromError key once
        let from_error_key = StaticKey::Name(self.repository.strings.intern("fromError"));

        // scan receiver variants for missing static members
        let mut missing_from_error = false;
        match receiver_ty {
            Type::Reference { symbol, .. } => {
                let mut visited = Vec::new();
                let member = self.resolve_member_symbol_for_symbol(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.module.id,
                    ctx.profile,
                    &ctx.index,
                    ctx.tree,
                    ctx.symbols,
                    &*ctx.types,
                    *symbol,
                    &from_error_key,
                    MemberLookupMode::Value,
                    &mut visited,
                )?;
                if member.is_none() {
                    missing_from_error = true;
                }
            }
            Type::Union { elements } => {
                for element_id in elements {
                    let element_ty = ctx.types.get_type(*element_id);
                    if let Type::Reference { symbol, .. } = element_ty {
                        let mut visited = Vec::new();
                        let member = self.resolve_member_symbol_for_symbol(
                            ctx.compiler_context,
                            ctx.module,
                            ctx.module.id,
                            ctx.profile,
                            &ctx.index,
                            ctx.tree,
                            ctx.symbols,
                            &*ctx.types,
                            *symbol,
                            &from_error_key,
                            MemberLookupMode::Value,
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
            let _ = self.report_missing_member_diagnostic(
                ctx.type_view(),
                expression_id.into_any(),
                receiver_ty_id,
                from_error_key,
                false,
            )?;
        }

        Ok(())
    }

    /// Ensure the enclosing function return type can accept a Try error.
    fn ensure_try_return_type_assignable(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        error_ty_id: LocalTypeId,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        // require a return type in the current function
        let Some(return_ty_id) = state.return_type else {
            self.error(AnalyzeError::MissingTryReturnType {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(());
        };

        // require a Try return type
        let return_ty = ctx.types.get_type(return_ty_id).clone();
        if !self.is_interface_implemented(ctx.symbol_type_view(), &return_ty, LanguageSymbol::Try) {
            self.error(AnalyzeError::MissingTryReturnType {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(());
        }

        // resolve value and error types from the return type
        let value_and_error_types = self.try_value_and_error_types_from_receiver(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            return_ty_id,
            &return_ty,
        )?;

        let Some((value_ty_id, return_error_ty_id)) = value_and_error_types else {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(());
        };

        // resolve the return type branch signature
        let branch = self.resolve_try_branch_member(
            &mut ctx.reborrow(),
            expression_id,
            expression_id,
            return_ty_id,
            &return_ty,
        )?;

        // reject missing branch members on the return type
        if !branch.resolved.has_member {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(());
        }

        // validate the branch return type against the return value and error types
        let is_valid = self.check_try_branch_return_type_assignable(
            &mut ctx.reborrow(),
            expression_id,
            &branch.resolved,
            value_ty_id,
            return_error_ty_id,
        )?;
        if !is_valid {
            self.error(AnalyzeError::InvalidTryBranch {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(());
        }

        // relate propagated error to the return error type
        ctx.infer.push_constraint(Constraint::Subtype {
            sub_type: error_ty_id,
            super_type: return_error_ty_id,
            variance: None,
        });

        // enforce propagated error compatibility after convergence when needed
        self.enforce_assignability_or_defer_diagnostic(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            return_error_ty_id,
            error_ty_id,
            UnassignableRelationFailureMode::ReportAndContinue,
        )?;

        Ok(())
    }

    /// Warn when Try error types do not implement Error.
    fn warn_try_error_type(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        error_ty_id: LocalTypeId,
    ) {
        // NOTE #Cleanup: move this warning to the lint pipeline once available
        // only lint user modules
        if !matches!(ctx.module.source, ModuleSource::User) {
            return;
        }

        // skip unknown or inference-driven error types
        let error_ty = ctx.types.get_type(error_ty_id);
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
        let error_symbol = self.language_symbol(ctx.profile, LanguageSymbol::Error);
        let error_reference_id = ctx.types.insert_type_from_any(
            Type::Reference {
                symbol: error_symbol,
                static_arguments: None,
            },
            expression_id.into_any(),
        );

        // warn when error type is not assignable to Error
        if self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
            error_reference_id,
            error_ty_id,
        ) == Assignability::NotAssignable
        {
            self.warning(AnalyzeWarning::TryErrorNotError {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                ty: error_ty_id.into_global(ctx.module.id),
            });
        }
    }

    /// Split a type into Try and non-Try elements.
    fn split_try_elements(
        &self,
        ctx: &InferContext<'_>,
        ty_id: LocalTypeId,
    ) -> (Vec<LocalTypeId>, Vec<LocalTypeId>) {
        // collect Try and non-Try elements
        let mut try_elements = Vec::new();
        let mut non_try_elements = Vec::new();

        // split based on interface implementation
        match ctx.types.get_type(ty_id) {
            Type::Union { elements } => {
                for element_id in elements {
                    let element_ty = ctx.types.get_type(*element_id);
                    if self.is_interface_implemented(
                        ctx.symbol_type_view(),
                        element_ty,
                        LanguageSymbol::Try,
                    ) {
                        try_elements.push(*element_id);
                    } else {
                        non_try_elements.push(*element_id);
                    }
                }
            }
            ty => {
                if self.is_interface_implemented(ctx.symbol_type_view(), ty, LanguageSymbol::Try) {
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
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        element_ids: &[LocalTypeId],
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // collect the success value types
        let mut value_types = Vec::new();

        // resolve branch types for each Try element
        for element_id in element_ids {
            let element_ty = ctx.types.get_type(*element_id).clone();
            let branch = self.resolve_try_branch_member(
                &mut ctx.reborrow(),
                expression_id,
                receiver_expression_id,
                *element_id,
                &element_ty,
            )?;

            // reject missing branches for Try elements
            if !branch.resolved.has_member {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    *element_id,
                );
                continue;
            }

            // resolve value and error types for the receiver and branch
            let value_and_error_types = self.resolve_try_value_and_error_types(
                &mut ctx.reborrow(),
                expression_id,
                receiver_expression_id.into_any(),
                *element_id,
                &element_ty,
                &branch.resolved,
            )?;
            let Some((value_ty_id, _error_ty_id)) = value_and_error_types else {
                self.error(AnalyzeError::InvalidTryBranch {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
                continue;
            };

            // collect ok branch types and strip nullish
            let (non_nullish, _) = self.strip_nullish_from_union(value_ty_id, ctx.types);
            if let Some(non_nullish) = non_nullish {
                value_types.push(non_nullish);
            }
        }

        // return collected success types
        Ok(value_types)
    }

    fn try_branch_member_key(&self) -> StaticKey {
        let name_id = self.repository.strings.intern("branch");
        StaticKey::Name(name_id)
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
