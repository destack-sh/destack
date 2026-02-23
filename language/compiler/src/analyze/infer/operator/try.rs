use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_coalesce_expression(
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
            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                infer,
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

            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                infer,
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
                self.emit_no_overload_for_receiver_type(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    non_nullish_ty_id,
                    types,
                );
            } else {
                let value_and_error_types = self.resolve_try_value_and_error_types(
                    module,
                    expression_id,
                    left_id.into_any(),
                    non_nullish_ty_id,
                    &non_nullish_ty,
                    &branch.resolved,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                    infer,
                    &ctx.options,
                )?;

                if value_and_error_types.is_none() {
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
                        infer,
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

        self.record_provisional_builtin_resolution(
            expression_id.into_global_any(module.id),
            Some(left_ty_id),
            infer,
            types,
        );

        Ok(result_ty_id)
    }

    /// Infer an index access expression.

    pub(crate) fn infer_try_unwrap_expression(
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

        // handle Try unions by aggregating value and error types
        if let Type::Union { elements } = &left_ty {
            let mut value_types = Vec::new();
            let mut error_types = Vec::new();

            // validate each Try union element and collect value and error types
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
                    self.emit_no_overload_for_receiver_type(
                        module,
                        ctx.profile,
                        expression_id.into_any(),
                        left_ty_id,
                        types,
                    );
                    let ty = Type::Error;
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
                    self.emit_no_overload_for_receiver_type(
                        module,
                        ctx.profile,
                        expression_id.into_any(),
                        *element_id,
                        types,
                    );
                    let ty = Type::Error;
                    return Ok(types.insert_type_from(ty, expression_id));
                }

                // resolve value and error types for the receiver and branch
                let value_and_error_types = self.resolve_try_value_and_error_types(
                    module,
                    expression_id,
                    left_id.into_any(),
                    *element_id,
                    &element_ty,
                    &branch.resolved,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                    infer,
                    &ctx.options,
                )?;
                let Some((value_ty_id, error_ty_id)) = value_and_error_types else {
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

            // build union types for merged branch value and error types
            let unknown_placeholder_type = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let value_source = value_types.first().copied().unwrap_or_else(|| {
                types.insert_type_from(unknown_placeholder_type.clone(), expression_id)
            });
            let error_source = error_types
                .first()
                .copied()
                .unwrap_or_else(|| types.insert_type_from(unknown_placeholder_type, expression_id));
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
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                left_ty_id,
                types,
            );
            let ty = Type::Error;
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
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                left_ty_id,
                types,
            );
            let ty = Type::Error;
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve value and error types for the receiver and branch
        let value_and_error_types = self.resolve_try_value_and_error_types(
            module,
            expression_id,
            left_id.into_any(),
            left_ty_id,
            &left_ty,
            &branch.resolved,
            ctx.profile,
            tree,
            symbols,
            types,
            infer,
            &ctx.options,
        )?;
        let Some((value_ty_id, error_ty_id)) = value_and_error_types else {
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
            self.ensure_try_return_type_assignable(
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
        self.record_try_branch_resolution(module, expression_id, left_ty_id, &branch, infer, types);

        Ok(value_ty_id)
    }

    /// Resolve Try value and error types from receiver static arguments.
    fn try_value_and_error_types_from_receiver(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        infer: &InferTable,
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
            Some(receiver_ty_id),
            receiver_ty,
            infer,
            options,
            tree,
            symbols,
            types,
        )?;

        // require at least two static arguments for Try value and error types
        let Some(value_argument) = inherited.arguments.first() else {
            return Ok(None);
        };
        let Some(error_argument) = inherited.arguments.get(1) else {
            return Ok(None);
        };

        // convert static arguments into value and error types
        let value_ty_id = self.convert_static_argument_type(value_argument, receiver_id, types);
        let error_ty_id = self.convert_static_argument_type(error_argument, receiver_id, types);

        Ok(Some((value_ty_id, error_ty_id)))
    }

    /// Resolve Try value and error types from the branch return shape.
    fn try_branch_value_and_error_types_from_return(
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
            self.resolve_declared_type(module, profile, return_ty_id, tree, symbols, types)?;
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
                let canonical_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let try_branch_symbol = self.language_symbol(profile, LanguageSymbol::TryBranch);
                if canonical_symbol == try_branch_symbol {
                    let options = self.analyze_context_options_for_module(module.id);
                    let resolved_arguments = self.resolve_type_reference_static_arguments(
                        module,
                        profile,
                        expression_id.into_any(),
                        canonical_symbol,
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

                if canonical_symbol.ty() != SymbolType::TypeAlias {
                    return Ok(None);
                }

                let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
                    module,
                    profile,
                    canonical_symbol,
                    expression_id.into_any(),
                    symbols,
                    types,
                ) else {
                    return Ok(None);
                };

                let options = self.analyze_context_options_for_module(module.id);
                let resolved_arguments = self.resolve_type_reference_static_arguments(
                    module,
                    profile,
                    expression_id.into_any(),
                    canonical_symbol,
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
                        canonical_symbol,
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
                    self.resolve_declared_type(module, profile, alias_ty_id, tree, symbols, types)?;
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

        // collect ok/error value types from union members
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

    /// Resolve Try value and error types while validating branch assignability.
    fn resolve_try_value_and_error_types(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        branch: &ResolvedMemberFunction,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<Option<(LocalTypeId, LocalTypeId)>> {
        // resolve value and error types from receiver static arguments when available
        let receiver_value_and_error_types = self.try_value_and_error_types_from_receiver(
            module,
            profile,
            receiver_id,
            receiver_ty_id,
            receiver_ty,
            infer,
            options,
            tree,
            symbols,
            types,
        )?;

        // resolve value and error types from the branch return shape
        let branch_value_and_error_types = if let Some(return_ty_id) = branch.signature.return_type
        {
            self.try_branch_value_and_error_types_from_return(
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

        // prefer receiver value and error types when they are concrete
        let mut value_and_error_types = receiver_value_and_error_types;
        if let Some(receiver_value_and_error_types) = receiver_value_and_error_types
            && self.try_value_and_error_types_need_branch(receiver_value_and_error_types, types)
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
            && !self.try_value_and_error_types_need_branch(receiver_value_and_error_types, types)
        {
            let is_valid = self.check_try_branch_return_type_assignable(
                module,
                expression_id,
                branch,
                receiver_value_and_error_types.0,
                receiver_value_and_error_types.1,
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
            Some(receiver_ty_id),
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
                module,
                profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
        }

        // register instance if needed
        let member_instance_id = self.record_member_call_instance_id(
            module,
            profile,
            expression_id,
            &resolved,
            tree,
            symbols,
            infer,
            types,
        )?;

        Ok(TryBranchMember {
            resolved,
            member_instance_id,
        })
    }

    /// Validate the Try branch return type against receiver value and error types.
    fn check_try_branch_return_type_assignable(
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
            self.resolve_declared_type(module, profile, return_ty_id, tree, symbols, types)?;
        }

        // skip additional diagnostics when the branch already errors
        if matches!(types.get_type(return_ty_id), Type::Error) {
            return Ok(true);
        }

        // extract value and error types from the branch return shape
        let Some((branch_value_ty_id, branch_error_ty_id)) = self
            .try_branch_value_and_error_types_from_return(
                module,
                expression_id,
                return_ty_id,
                profile,
                tree,
                symbols,
                types,
            )?
        else {
            let is_try_branch = matches!(types.get_type(return_ty_id), Type::Reference { symbol, .. } if {
                let canonical = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    *symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                canonical == self.language_symbol(profile, LanguageSymbol::TryBranch)
            });
            return Ok(is_try_branch);
        };

        // compare value and error types to the expected Try arguments when they are concrete
        let branch_value_is_parameter = matches!(
            types.get_type(branch_value_ty_id),
            Type::Reference { symbol, .. }
                if self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        );
        let branch_error_is_parameter = matches!(
            types.get_type(branch_error_ty_id),
            Type::Reference { symbol, .. }
                if self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        );

        if self.should_check_try_value_error_assignability(value_ty_id, types)
            && !branch_value_is_parameter
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

        if self.should_check_try_value_error_assignability(error_ty_id, types)
            && !branch_error_is_parameter
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
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) {
        self.record_provisional_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &branch.resolved.member_resolution,
            branch.member_instance_id,
            None,
            branch.resolved.has_member,
            infer,
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
        types: &mut TypeTable,
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
            self.emit_missing_member_diagnostic_for_receiver_type(
                module,
                profile,
                expression_id,
                receiver_ty_id,
                from_error_key,
                symbols,
                types,
                false,
            )?;
        }

        Ok(())
    }

    /// Ensure the enclosing function return type can accept a Try error.
    fn ensure_try_return_type_assignable(
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

        // resolve value and error types from the return type
        let value_and_error_types = match self.try_value_and_error_types_from_receiver(
            module,
            ctx.profile,
            expression_id.into_any(),
            return_ty_id,
            &return_ty,
            infer,
            &ctx.options,
            tree,
            symbols,
            types,
        ) {
            Ok(value_and_error_types) => value_and_error_types,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        let Some((value_ty_id, return_error_ty_id)) = value_and_error_types else {
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

        // validate the branch return type against the return value and error types
        let is_valid = match self.check_try_branch_return_type_assignable(
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

        // enforce propagated error compatibility after convergence when needed
        let assignability_check = self.enforce_assignability_or_defer_unassignable_diagnostic(
            module,
            ctx.profile,
            expression_id.into_any(),
            return_error_ty_id,
            error_ty_id,
            symbols,
            types,
            infer,
            &ctx.options,
            UnassignableRelationFailureMode::ReportAndContinue,
        );
        if let Err(error) = assignability_check {
            self.error(error);
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
        // NOTE #Cleanup: move this warning to the lint pipeline once available
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
        let error_symbol = self.language_symbol(ctx.profile, LanguageSymbol::Error);
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
                self.emit_no_overload_for_receiver_type(
                    module,
                    profile,
                    expression_id.into_any(),
                    *element_id,
                    types,
                );
                continue;
            }

            // resolve value and error types for the receiver and branch
            let value_and_error_types = self.resolve_try_value_and_error_types(
                module,
                expression_id,
                receiver_expression_id.into_any(),
                *element_id,
                &element_ty,
                &branch.resolved,
                profile,
                tree,
                symbols,
                types,
                infer,
                &ctx.options,
            )?;
            let Some((value_ty_id, _error_ty_id)) = value_and_error_types else {
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

    fn try_branch_member_key(&self) -> StaticKey {
        let name_id = self.program.strings.intern("branch");
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
