use super::*;
use crate::analyze::common::TypeContext;
use destack_dir::TypeIndexSignature;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_index_access_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        index_id: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // optional chain receivers must unwrap maybe before index lookup
        let optional_chain =
            self.infer_optional_chain_receiver(&mut ctx.reborrow(), receiver_id, state)?;
        let (receiver_id, receiver_ty_id, optional_chain_has_nullish) =
            if let Some(optional_chain) = optional_chain {
                let Some(receiver_ty_id) = optional_chain.receiver_ty_id else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    };
                    return Ok(ctx.types.insert_type_from(ty, expression_id));
                };
                (
                    optional_chain.receiver_id,
                    receiver_ty_id,
                    optional_chain.has_nullish,
                )
            } else {
                let receiver_ty_id =
                    self.infer_expression(&mut ctx.reborrow(), receiver_id, state)?;
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
        let receiver_ty_id = self.normalize_apparent_type(
            &mut ctx.type_context_reborrow(),
            receiver_ty_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let receiver_ty = ctx.types.get_type(receiver_ty_id).clone();

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
            let type_id = ctx.types.insert_type_from(ty, expression_id);
            return Ok(finish_result(type_id, ctx.types));
        }

        // ensure instance types for reference receivers
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            receiver_ty_id,
        )?;

        // resolve index expression and literal string when possible
        let (index_ty_id, literal_string, literal_integer, static_key) =
            if let Some(index_id) = index_id {
                let index_ty_id = self.infer_expression(&mut ctx.reborrow(), index_id, state)?;
                let (literal_string, literal_integer) = match ctx.tree.get(index_id) {
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::String(name_id),
                    } => (Some(self.repository.strings.get(*name_id)), None),
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::Integer(value),
                    } => (None, Some(*value)),
                    _ => (None, None),
                };
                let static_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    Key::Expression(index_id),
                );
                (
                    Some(index_ty_id),
                    literal_string,
                    literal_integer,
                    static_key,
                )
            } else {
                (None, None, None, None)
            };

        // reject computed property access when configured
        if state.options.no_computed_property_access
            && matches!(ctx.module.source, ModuleSource::User)
            && let Some(index_id) = index_id
        {
            let static_key = self.static_key_from_key(
                ctx.compiler_context.revision(),
                ctx.profile,
                ctx.tree,
                ctx.symbols,
                ctx.types,
                Key::Expression(index_id),
            );
            if static_key.is_none() {
                self.error(AnalyzeError::ComputedPropertyAccessDisabled {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                let type_id = ctx.types.insert_type_from(ty, expression_id);
                return Ok(finish_result(type_id, ctx.types));
            }
        }

        // handle builtin index access
        let builtin_ty_id = self.infer_builtin_index_access(
            &receiver_ty,
            receiver_ty_id,
            index_ty_id,
            literal_string.as_deref(),
            literal_integer,
            static_key.as_ref(),
            ctx.types,
            state.options.no_unchecked_indexed_access,
        );
        if let Some(builtin_ty_id) = builtin_ty_id {
            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                ctx.infer,
                ctx.types,
            );
            return Ok(finish_result(builtin_ty_id, ctx.types));
        }

        // treat indexed access with a literal key as a property lookup
        if let Some(static_key) = static_key.as_ref() {
            let mut visited = Vec::new();
            if let Some(member_ty_id) = self.infer_member_of_type(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                &receiver_ty,
                static_key,
                MemberLookupMode::Any,
                &mut visited,
            )? {
                return Ok(finish_result(member_ty_id, ctx.types));
            }
        }

        // guard non indexable receivers
        if !self.is_interface_implemented(
            ctx.symbol_type_view(),
            &receiver_ty,
            LanguageSymbol::Index,
        ) {
            if self.receiver_prefers_overload_diagnostic(&receiver_ty, ctx.types) {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    receiver_ty_id,
                );
            } else {
                self.error(AnalyzeError::NonIndexable {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
            let type_id = ctx.types.insert_type_from(Type::Error, expression_id);
            return Ok(finish_result(type_id, ctx.types));
        }

        // resolve the index member function
        let member_key = self.operator_member_key(LanguageSymbol::Index);
        let Some(resolved) = ({
            self.resolve_member_function(
                &mut ctx.reborrow(),
                expression_id,
                receiver_id,
                Some(receiver_ty_id),
                &receiver_ty,
                &member_key,
            )?
        }) else {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
            let type_id = ctx.types.insert_type_from(Type::Error, expression_id);
            return Ok(finish_result(type_id, ctx.types));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                receiver_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
            let type_id = ctx.types.insert_type_from(Type::Error, expression_id);
            return Ok(finish_result(type_id, ctx.types));
        }

        // resolve index parameter type
        let parameter_ty_id = resolved.signature.parameters.first().copied();
        if resolved.signature.parameters.len() != 1 {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                receiver_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
            let type_id = ctx.types.insert_type_from(Type::Error, expression_id);
            return Ok(finish_result(type_id, ctx.types));
        }

        // check index argument assignability
        if let (Some(parameter_ty_id), Some(index_ty_id)) = (parameter_ty_id, index_ty_id) {
            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: index_ty_id,
                super_type: parameter_ty_id,
                variance: None,
            });

            self.enforce_assignability_or_defer_diagnostic(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                parameter_ty_id,
                index_ty_id,
                UnassignableRelationFailureMode::PropagateError,
            )?;
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(
            &mut ctx.reborrow(),
            expression_id,
            receiver_ty_id,
            &resolved,
        )?;

        // resolve return type
        let value_ty_id = resolved.signature.return_type.unwrap_or_else(|| {
            ctx.types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            )
        });

        Ok(finish_result(value_ty_id, ctx.types))
    }

    /// Infer an index assignment expression.
    pub(crate) fn infer_index_assignment_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        index_expression_id: LocalNodeId<Expression>,
        value_expression_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // extract receiver and index expressions
        let Expression::Index {
            left: receiver_id,
            right: index_id,
        } = ctx.tree.get(index_expression_id)
        else {
            unreachable!("index assignment expects an index expression");
        };

        // resolve receiver type
        let receiver_ty_id = self.infer_expression(&mut ctx.reborrow(), *receiver_id, state)?;
        let receiver_ty_id = self.normalize_apparent_type(
            &mut ctx.type_context_reborrow(),
            receiver_ty_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let receiver_ty = ctx.types.get_type(receiver_ty_id).clone();

        // reject assignments through immutable references
        if self.type_is_immutable_reference(receiver_ty_id, ctx.types) {
            self.error(AnalyzeError::ImmutableReferenceAssignment {
                node: receiver_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // resolve index expression and literal string when possible
        let (index_ty_id, literal_string, literal_integer, static_key) =
            if let Some(index_id) = index_id {
                let index_ty_id = self.infer_expression(&mut ctx.reborrow(), *index_id, state)?;
                let (literal_string, literal_integer) = match ctx.tree.get(*index_id) {
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::String(name_id),
                    } => (Some(self.repository.strings.get(*name_id)), None),
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::Integer(value),
                    } => (None, Some(*value)),
                    _ => (None, None),
                };
                let static_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    Key::Expression(*index_id),
                );
                (
                    Some(index_ty_id),
                    literal_string,
                    literal_integer,
                    static_key,
                )
            } else {
                (None, None, None, None)
            };

        // reject writes to readonly index targets
        if self.index_access_is_readonly(
            &mut ctx.type_context_reborrow(),
            receiver_ty_id,
            index_ty_id,
            literal_string.as_deref(),
        ) {
            let member_key = static_key.unwrap_or_else(|| {
                let name_id = self.repository.strings.intern("<index>");
                StaticKey::Name(name_id)
            });
            self.error(AnalyzeError::ReadonlyProperty {
                node: index_expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                member_key,
            });
        }

        // reject computed property access when configured
        if state.options.no_computed_property_access
            && matches!(ctx.module.source, ModuleSource::User)
            && let Some(index_id) = *index_id
        {
            let static_key = self.static_key_from_key(
                ctx.compiler_context.revision(),
                ctx.profile,
                ctx.tree,
                ctx.symbols,
                ctx.types,
                Key::Expression(index_id),
            );
            if static_key.is_none() {
                self.error(AnalyzeError::ComputedPropertyAccessDisabled {
                    node: expression_id
                        .into_global_any(ctx.module.id)
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
            literal_integer,
            static_key.as_ref(),
            ctx.types,
            false,
        );
        if let Some(builtin_value_ty_id) = builtin_value_ty_id {
            let mut value_ctx = state.fork().with_expected_type(Some(builtin_value_ty_id));
            let value_ty_id =
                self.infer_expression(&mut ctx.reborrow(), value_expression_id, &mut value_ctx)?;

            // enforce explicit ownership when implicit managed values are disabled
            self.check_no_implicit_managed_value(
                ctx.module_type_view(),
                value_expression_id,
                builtin_value_ty_id,
                value_ty_id,
                ctx.tree,
                &state.options,
            );

            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: value_ty_id,
                super_type: builtin_value_ty_id,
                variance: None,
            });

            self.enforce_assignability_or_defer_diagnostic(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                builtin_value_ty_id,
                value_ty_id,
                UnassignableRelationFailureMode::PropagateError,
            )?;

            self.record_provisional_builtin_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                ctx.infer,
                ctx.types,
            );

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // guard non indexable receivers
        if !self.is_interface_implemented(
            ctx.symbol_type_view(),
            &receiver_ty,
            LanguageSymbol::IndexSet,
        ) {
            if self.receiver_prefers_overload_diagnostic(&receiver_ty, ctx.types) {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    receiver_ty_id,
                );
            } else {
                self.error(AnalyzeError::NonIndexable {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // resolve the index set member function
        let member_key = self.operator_member_key(LanguageSymbol::IndexSet);
        let Some(resolved) = ({
            self.resolve_member_function(
                &mut ctx.reborrow(),
                expression_id,
                *receiver_id,
                Some(receiver_ty_id),
                &receiver_ty,
                &member_key,
            )?
        }) else {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        };

        // handle missing member
        if !resolved.has_member {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                receiver_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // resolve index set parameter types
        let key_param_ty_id = resolved.signature.parameters.first().copied();
        let value_param_ty_id = resolved.signature.parameters.get(1).copied();
        if resolved.signature.parameters.len() != 2 {
            self.record_member_call_resolution(
                &mut ctx.reborrow(),
                expression_id,
                receiver_ty_id,
                &resolved,
            )?;
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                receiver_ty_id,
            );
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // check key argument assignability
        if let (Some(key_param_ty_id), Some(index_ty_id)) = (key_param_ty_id, index_ty_id) {
            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: index_ty_id,
                super_type: key_param_ty_id,
                variance: None,
            });
        }

        // infer value expression with contextual typing
        let mut value_ctx = state.fork().with_expected_type(value_param_ty_id);
        let value_ty_id =
            self.infer_expression(&mut ctx.reborrow(), value_expression_id, &mut value_ctx)?;

        // enforce explicit ownership when implicit managed values are disabled
        if let Some(value_param_ty_id) = value_param_ty_id {
            self.check_no_implicit_managed_value(
                ctx.module_type_view(),
                value_expression_id,
                value_param_ty_id,
                value_ty_id,
                ctx.tree,
                &state.options,
            );
        }

        // check value argument assignability
        if let Some(value_param_ty_id) = value_param_ty_id {
            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: value_ty_id,
                super_type: value_param_ty_id,
                variance: None,
            });

            self.enforce_assignability_or_defer_diagnostic(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                value_param_ty_id,
                value_ty_id,
                UnassignableRelationFailureMode::PropagateError,
            )?;
        }

        // finalize resolution and instance registration
        self.record_member_call_resolution(
            &mut ctx.reborrow(),
            expression_id,
            receiver_ty_id,
            &resolved,
        )?;

        // return void for index assignment
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer a try unwrap expression.
    fn infer_builtin_index_access(
        &self,
        receiver_ty: &Type,
        receiver_ty_id: LocalTypeId,
        index_ty_id: Option<LocalTypeId>,
        literal_string: Option<&str>,
        literal_integer: Option<i64>,
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
                self.union_type_from_list(vec![ty_id, undefined_ty_id], ty_id, types)
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
                    literal_integer,
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
                if let Some(index) = literal_integer
                    && index >= 0
                {
                    let index = index as usize;
                    if index < elements.len() {
                        return Some(elements[index].ty);
                    }
                }
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
                    literal_integer,
                    static_key,
                    types,
                    include_undefined,
                )
            }
            _ => None,
        }
    }

    /// Return true when missing index contracts should report as overload misses.
    fn receiver_prefers_overload_diagnostic(&self, receiver_ty: &Type, types: &TypeTable) -> bool {
        match receiver_ty {
            Type::Reference { symbol, .. } => matches!(
                symbol.ty(),
                SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype
            ),
            Type::This => true,
            Type::Value { value } | Type::ReferenceOf { right: value, .. } => {
                self.receiver_prefers_overload_diagnostic(types.get_type(*value), types)
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element_id| {
                    self.receiver_prefers_overload_diagnostic(types.get_type(*element_id), types)
                })
            }
            _ => false,
        }
    }

    fn infer_index_signature_access(
        &self,
        index_signatures: &[TypeIndexSignature],
        index_ty_id: LocalTypeId,
        literal_string: Option<&str>,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let key_kind =
            index_key_kind_for_index(index_ty_id, literal_string, types, &self.repository.strings)?;
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
        ctx: &mut TypeContext<'_>,
        receiver_ty_id: LocalTypeId,
        index_ty_id: Option<LocalTypeId>,
        literal_string: Option<&str>,
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

            let current_ty = ctx.types.get_type(current_id).clone();
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
                        } = ctx.types.get_type(index_ty_id)
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
                        ctx.types,
                        &self.repository.strings,
                    ) else {
                        continue;
                    };
                    for signature in index_signatures {
                        let signature_kind = index_key_kind_for_type(signature.key_type, ctx.types);
                        if index_key_kinds_compatible_for_access(signature_kind, key_kind) {
                            found = true;
                            if signature.is_readonly {
                                is_readonly = true;
                            }
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = ctx.types.get_instance_type_id(symbol) {
                        pending.push(instance_id);
                        continue;
                    }
                    let source_id = ctx.types.get_type_source(current_id);
                    if let Some(apparent_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
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
