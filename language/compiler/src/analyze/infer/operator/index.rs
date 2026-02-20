use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_index_access_expression(
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
        // optional chain receivers must unwrap maybe before index lookup
        let optional_chain = self.infer_optional_chain_receiver(
            module,
            receiver_id,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;
        let (receiver_id, receiver_ty_id, optional_chain_has_nullish) =
            if let Some(optional_chain) = optional_chain {
                let Some(receiver_ty_id) = optional_chain.receiver_ty_id else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                };
                (
                    optional_chain.receiver_id,
                    receiver_ty_id,
                    optional_chain.has_nullish,
                )
            } else {
                let receiver_ty_id =
                    self.infer_expression(module, receiver_id, tree, symbols, types, infer, ctx)?;
                (receiver_id, receiver_ty_id, false)
            };
        let finish_result = |type_id: LocalTypeId, types: &mut TypeTable| {
            self.optional_chain_result_type(
                expression_id,
                type_id,
                optional_chain_has_nullish,
                types,
            )
        };

        // resolve receiver type
        let options = ctx.options;
        let receiver_ty_id = self.normalize_apparent_type(
            module,
            ctx.profile,
            receiver_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
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
            let type_id = types.insert_type_from(ty, expression_id);
            return Ok(finish_result(type_id, types));
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
        let (index_ty_id, literal_string, static_key) = if let Some(index_id) = index_id {
            let index_ty_id =
                self.infer_expression(module, index_id, tree, symbols, types, infer, ctx)?;
            let literal_string = match tree.get(index_id) {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(name_id),
                } => Some(self.program.strings.get(*name_id)),
                _ => None,
            };
            let static_key = self.static_key_from_dynamic_key(
                ctx.profile,
                DynamicKey::Expression(index_id),
                tree,
                symbols,
                types,
            );
            (Some(index_ty_id), literal_string, static_key)
        } else {
            (None, None, None)
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
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                let type_id = types.insert_type_from(ty, expression_id);
                return Ok(finish_result(type_id, types));
            }
        }

        // handle builtin index access
        let builtin_ty_id = self.infer_builtin_index_access(
            &receiver_ty,
            receiver_ty_id,
            index_ty_id,
            literal_string.as_deref(),
            static_key.as_ref(),
            types,
            options.no_unchecked_indexed_access,
        );
        if let Some(builtin_ty_id) = builtin_ty_id {
            self.commit_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                types,
            );
            return Ok(finish_result(builtin_ty_id, types));
        }

        // treat indexed access with a literal key as a property lookup
        if let Some(static_key) = static_key.as_ref() {
            let mut visited = Vec::new();
            if let Some(member_ty_id) = self.infer_member_of_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                symbols,
                &receiver_ty,
                static_key,
                MemberLookupMode::Any,
                types,
                &mut visited,
            )? {
                return Ok(finish_result(member_ty_id, types));
            }
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
            let type_id = types.insert_type_from(ty, expression_id);
            return Ok(finish_result(type_id, types));
        }

        // resolve the index member function
        let member_key = self.operator_member_key(LanguageSymbol::Index);
        let Some(resolved) = self.resolve_member_function(
            module,
            expression_id,
            receiver_id,
            Some(receiver_ty_id),
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
            let _reported = self.report_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let type_id = types.insert_type_from(ty, expression_id);
            return Ok(finish_result(type_id, types));
        };

        // handle missing member
        if !resolved.has_member {
            self.commit_member_call_resolution(
                module,
                ctx.profile,
                expression_id,
                receiver_ty_id,
                &resolved,
                tree,
                symbols,
                infer,
                types,
            )?;
            let _reported = self.report_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let type_id = types.insert_type_from(ty, expression_id);
            return Ok(finish_result(type_id, types));
        }

        // resolve index parameter type
        let parameter_ty_id = resolved.signature.dynamic_parameters.first().copied();
        if resolved.signature.dynamic_parameters.len() != 1 {
            let _reported = self.report_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
        }

        // check index argument assignability
        if let (Some(parameter_ty_id), Some(index_ty_id)) = (parameter_ty_id, index_ty_id) {
            infer.push_constraint(Constraint::Subtype {
                sub_type: index_ty_id,
                super_type: parameter_ty_id,
                variance: None,
            });

            if !self.is_type_assignable_or_deferred(
                module,
                ctx.profile,
                symbols,
                parameter_ty_id,
                index_ty_id,
                types,
                &options,
            ) {
                if let Some(error) = self.unassignable_type_error_for_types(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    parameter_ty_id,
                    index_ty_id,
                    types,
                ) {
                    return Err(error);
                }
            }
        }

        // finalize resolution and instance registration
        self.commit_member_call_resolution(
            module,
            ctx.profile,
            expression_id,
            receiver_ty_id,
            &resolved,
            tree,
            symbols,
            infer,
            types,
        )?;

        // resolve return type
        let value_ty_id = resolved.signature.return_type.unwrap_or_else(|| {
            types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            )
        });

        Ok(finish_result(value_ty_id, types))
    }

    /// Infer an index assignment expression.
    pub(crate) fn infer_index_assignment_expression(
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
        let receiver_ty_id = self.normalize_apparent_type(
            module,
            ctx.profile,
            receiver_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let receiver_ty = types.get_type(receiver_ty_id).clone();

        // reject assignments through immutable references
        if self.type_is_immutable_reference(receiver_ty_id, types) {
            self.error(AnalyzeError::ImmutableReferenceAssignment {
                node: receiver_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // resolve index expression and literal string when possible
        let (index_ty_id, literal_string, static_key) = if let Some(index_id) = index_id {
            let index_ty_id =
                self.infer_expression(module, *index_id, tree, symbols, types, infer, ctx)?;
            let literal_string = match tree.get(*index_id) {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(name_id),
                } => Some(self.program.strings.get(*name_id)),
                _ => None,
            };
            let static_key = self.static_key_from_dynamic_key(
                ctx.profile,
                DynamicKey::Expression(*index_id),
                tree,
                symbols,
                types,
            );
            (Some(index_ty_id), literal_string, static_key)
        } else {
            (None, None, None)
        };

        // reject writes to readonly index targets
        if self.index_access_is_readonly(
            module,
            ctx.profile,
            receiver_ty_id,
            index_ty_id,
            literal_string.as_deref(),
            symbols,
            types,
        ) {
            let member_key = static_key.unwrap_or_else(|| {
                let name_id = self.program.strings.intern("<index>");
                StaticKey::Name(name_id)
            });
            self.error(AnalyzeError::ReadonlyProperty {
                node: index_expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                member_key,
            });
        }

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
            static_key.as_ref(),
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

            if !self.is_type_assignable_or_deferred(
                module,
                ctx.profile,
                symbols,
                builtin_value_ty_id,
                value_ty_id,
                types,
                &options,
            ) {
                if let Some(error) = self.unassignable_type_error_for_types(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    builtin_value_ty_id,
                    value_ty_id,
                    types,
                ) {
                    return Err(error);
                }
            }

            self.commit_builtin_resolution(
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
            Some(receiver_ty_id),
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
            let _reported = self.report_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.commit_member_call_resolution(
                module,
                ctx.profile,
                expression_id,
                receiver_ty_id,
                &resolved,
                tree,
                symbols,
                infer,
                types,
            )?;
            let _reported = self.report_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve index set parameter types
        let key_param_ty_id = resolved.signature.dynamic_parameters.first().copied();
        let value_param_ty_id = resolved.signature.dynamic_parameters.get(1).copied();
        if resolved.signature.dynamic_parameters.len() != 2 {
            let _reported = self.report_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id,
                types,
            );
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

            if !self.is_type_assignable_or_deferred(
                module,
                ctx.profile,
                symbols,
                value_param_ty_id,
                value_ty_id,
                types,
                &options,
            ) {
                if let Some(error) = self.unassignable_type_error_for_types(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    value_param_ty_id,
                    value_ty_id,
                    types,
                ) {
                    return Err(error);
                }
            }
        }

        // finalize resolution and instance registration
        self.commit_member_call_resolution(
            module,
            ctx.profile,
            expression_id,
            receiver_ty_id,
            &resolved,
            tree,
            symbols,
            infer,
            types,
        )?;

        // return void for index assignment
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a try unwrap expression.

    fn infer_builtin_index_access(
        &self,
        receiver_ty: &Type,
        receiver_ty_id: LocalTypeId,
        index_ty_id: Option<LocalTypeId>,
        literal_string: Option<&str>,
        static_key: Option<&StaticKey>,
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
            Type::ReferenceOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.infer_builtin_index_access(
                    &inner_ty,
                    *right,
                    index_ty_id,
                    literal_string,
                    static_key,
                    types,
                    include_undefined,
                )
            }
            Type::Array { element, .. } => {
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
            Type::ArraySized { element, .. } => Some(*element),
            Type::Tuple { elements, .. } => {
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

                if let Some(key) = static_key
                    && let Some(field) = fields.iter().find(|field| field.key.matches(key))
                {
                    return Some(field.ty);
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
                    static_key,
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

    /// Check whether an index access target is readonly for assignment.
    fn index_access_is_readonly(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_ty_id: LocalTypeId,
        index_ty_id: Option<LocalTypeId>,
        literal_string: Option<&str>,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let mut found = false;
        let mut is_readonly = false;
        let mut pending = vec![receiver_ty_id];
        let mut visited = Vec::new();

        // walk union/alias shapes to resolve readonly indexability
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);

            let current_ty = types.get_type(current_id).clone();
            match current_ty {
                Type::Value { value } => {
                    pending.push(value);
                }
                Type::ReferenceOf { right, .. } => {
                    pending.push(right);
                }
                Type::Union { elements } | Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(element_id);
                    }
                }
                Type::Array {
                    is_readonly: array_readonly,
                    ..
                }
                | Type::ArraySized {
                    is_readonly: array_readonly,
                    ..
                } => {
                    found = true;
                    if array_readonly {
                        is_readonly = true;
                    }
                }
                Type::Tuple {
                    elements,
                    is_readonly: tuple_readonly,
                } => {
                    found = true;
                    if tuple_readonly {
                        is_readonly = true;
                        continue;
                    }
                    if let Some(index_ty_id) = index_ty_id
                        && let Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(index)),
                        } = types.get_type(index_ty_id)
                        && *index >= 0
                    {
                        let index = *index as usize;
                        if let Some(element) = elements.get(index)
                            && element.is_readonly
                        {
                            is_readonly = true;
                        }
                    } else if elements.iter().any(|element| element.is_readonly) {
                        is_readonly = true;
                    }
                }
                Type::Object {
                    index_signatures, ..
                } => {
                    let Some(index_ty_id) = index_ty_id else {
                        continue;
                    };
                    let Some(key_kind) = index_key_kind_for_index(
                        index_ty_id,
                        literal_string,
                        types,
                        &self.program.strings,
                    ) else {
                        continue;
                    };
                    for signature in index_signatures {
                        let signature_kind = index_key_kind_for_type(signature.key_type, types);
                        if index_key_kinds_compatible_for_access(signature_kind, key_kind) {
                            found = true;
                            if signature.is_readonly {
                                is_readonly = true;
                            }
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = types.get_instance_type_id(symbol) {
                        pending.push(instance_id);
                        continue;
                    }
                    let source_id = types.get_type_source(current_id);
                    if let Some(apparent_id) = self
                        .apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                    {
                        pending.push(apparent_id);
                    }
                }
                _ => {}
            }
        }

        found && is_readonly
    }
}
