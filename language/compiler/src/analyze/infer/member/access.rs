use super::*;
use crate::analyze::common::{CanonicalSymbolMode, InferContext};
use destack_dir::{FunctionKind, GenericArgument, Property};
use destack_source::SourcePartKey;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a member access expression.
    pub(crate) fn infer_member_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        member_name: StringId,
        static_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_MEMBER);

        // query and normalize the receiver state
        let receiver = match self.query_member_access_receiver(
            &mut ctx.reborrow(),
            expression_id,
            left_id,
            state,
        )? {
            MemberAccessReceiverQuery::EarlyType(type_id) => return Ok(type_id),
            MemberAccessReceiverQuery::Receiver(receiver) => receiver,
        };
        let finish_result = |type_id: LocalTypeId, types: &mut TypeTable| {
            self.optional_chain_result_type(
                expression_id,
                type_id,
                receiver.has_optional_nullish,
                types,
            )
        };
        // short circuit member access on any
        if matches!(
            &receiver.receiver_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            }
        ) {
            let type_id = self.any_member_access_type(expression_id, &mut *ctx.types);
            return Ok(finish_result(type_id, &mut *ctx.types));
        }

        // resolve the lookup key from member syntax
        let member_key = StaticKey::Name(member_name);

        // handle enum field access early to preserve nominal enum types
        if let Some(enum_reference_id) = self.resolve_enum_field_access(
            &mut ctx.reborrow(),
            expression_id,
            receiver.receiver_id,
            receiver.receiver_ty_id,
            &receiver.receiver_ty,
            &member_key,
        )? {
            return Ok(finish_result(enum_reference_id, &mut *ctx.types));
        }

        // resolve member symbol and substitution state for this receiver
        let lookup = self.resolve_member_access_lookup(
            &mut ctx.reborrow(),
            expression_id,
            &receiver,
            member_key,
            state,
        )?;

        // infer and commit the member access type from resolved lookup state
        let resolved_member_ty_id = self.infer_member_access_type_from_lookup(
            &mut ctx.reborrow(),
            expression_id,
            static_arguments,
            &receiver,
            &lookup,
            state,
        )?;

        Ok(finish_result(resolved_member_ty_id, &mut *ctx.types))
    }

    /// Return true when the active function context uses lambda signature semantics.
    fn context_function_uses_lambda_signature(&self, state: &InferState, tree: &NodeTree) -> bool {
        let Some(function_id) = state.in_function else {
            return false;
        };

        match function_id.ty {
            NodeType::Declaration => {
                let declaration = tree.get(function_id.into_typed::<Declaration>());
                let Declaration::Function(declaration) = declaration else {
                    return false;
                };

                declaration.signature.kind == FunctionKind::Lambda
            }
            NodeType::Member => {
                let member = tree.get(function_id.into_typed::<Member>());
                let Member::Method { signature, .. } = member else {
                    return false;
                };

                signature.kind == FunctionKind::Lambda
            }
            NodeType::Property => {
                let property = tree.get(function_id.into_typed::<Property>());
                let Property::Method { signature, .. } = property else {
                    return false;
                };

                signature.kind == FunctionKind::Lambda
            }
            _ => false,
        }
    }

    /// Return true when the active function context is a non-lambda object-literal method.
    fn context_function_is_non_lambda_property_method(
        &self,
        state: &InferState,
        tree: &NodeTree,
    ) -> bool {
        let Some(function_id) = state.in_function else {
            return false;
        };
        if function_id.ty != NodeType::Property {
            return false;
        }

        let property = tree.get(function_id.into_typed::<Property>());
        let Property::Method { signature, .. } = property else {
            return false;
        };

        signature.kind != FunctionKind::Lambda
    }

    /// Resolve one canonical infer-time receiver type for `import.meta`.
    fn resolve_import_meta_receiver_type_for_infer(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        state: &InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(import_meta_symbol) =
            self.get_language_symbol(ctx.profile, LanguageSymbol::ImportMeta)
        else {
            return Ok(None);
        };

        if let Some(import_meta_instance_ty_id) = self
            .apparent_instance_type(
                &mut ctx.type_context_reborrow(),
                receiver_id.into_any(),
                import_meta_symbol,
            )
            .or(self.import_instance_type_for_symbol(
                &ctx.index,
                ctx.compiler_context.revision(),
                ctx.profile,
                receiver_id.into_any(),
                import_meta_symbol,
                ctx.types,
            )?)
        {
            return Ok(Some(import_meta_instance_ty_id));
        }

        if import_meta_symbol.module_id != ctx.module.id {
            let receiver_ty_id = self.resolve_remote_symbol_value_type_for_context(
                &mut ctx.reborrow(),
                state,
                receiver_id.into_any(),
                import_meta_symbol,
            )?;
            return Ok(Some(receiver_ty_id));
        }

        Ok(Some(ctx.types.insert_type_from(
            Type::Reference {
                symbol: import_meta_symbol,
                static_arguments: None,
            },
            receiver_id,
        )))
    }

    /// Recompute one receiver type from syntax without reusing inferred-expression cache.
    fn infer_member_receiver_type_without_cache(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let receiver_expression = ctx.tree.get(receiver_id);
        let receiver_ty_id = match receiver_expression {
            Expression::LocalReference {
                target_symbol,
                generic_arguments,
                ..
            }
            | Expression::ModuleReference {
                target_symbol,
                generic_arguments,
                ..
            }
            | Expression::GlobalReference {
                target_symbol,
                generic_arguments,
                ..
            } => Some(self.infer_reference_expression(
                &mut ctx.reborrow(),
                receiver_id,
                *target_symbol,
                Some(generic_arguments.as_slice()),
                state,
            )?),

            Expression::ImportMeta => {
                if let Some(receiver_ty_id) = self.resolve_import_meta_receiver_type_for_infer(
                    &mut ctx.reborrow(),
                    receiver_id,
                    state,
                )? {
                    Some(receiver_ty_id)
                } else {
                    self.error(AnalyzeError::Internal {
                        message: format!(
                            "missing language symbol for import.meta: module={}, profile={:?}",
                            ctx.module.id, state.profile,
                        ),
                    });
                    Some(ctx.types.insert_type_from(Type::Error, receiver_id))
                }
            }

            Expression::UnresolvedPath { .. }
            | Expression::PrivateIdentifier { .. }
            | Expression::This
            | Expression::Super => None,

            _ => None,
        };

        Ok(receiver_ty_id)
    }

    /// Query and normalize the receiver state for member access.
    fn query_member_access_receiver(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<MemberAccessReceiverQuery> {
        // optional chain receivers must unwrap maybe before member lookup
        let optional_chain =
            self.infer_optional_chain_receiver(&mut ctx.reborrow(), left_id, state)?;
        let (receiver_id, receiver_ty_id, has_optional_nullish) =
            if let Some(optional_chain) = optional_chain {
                let Some(receiver_ty_id) = optional_chain.receiver_ty_id else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    };
                    let undefined_ty_id = ctx.types.insert_type_from(ty, expression_id);
                    return Ok(MemberAccessReceiverQuery::EarlyType(undefined_ty_id));
                };
                (
                    optional_chain.receiver_id,
                    receiver_ty_id,
                    optional_chain.has_nullish,
                )
            } else {
                let is_projection_receiver =
                    self.is_projection_receiver_expression(&mut ctx.reborrow(), left_id);
                let receiver_ty_id = if is_projection_receiver {
                    let Expression::Type {
                        value: left_type, ..
                    } = ctx.tree.get(left_id)
                    else {
                        return Ok(MemberAccessReceiverQuery::EarlyType(
                            self.infer_expression(&mut ctx.reborrow(), left_id, state)?,
                        ));
                    };

                    self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *left_type,
                        true,
                        true,
                    )?
                } else {
                    self.infer_expression(&mut ctx.reborrow(), left_id, state)?
                };
                (left_id, receiver_ty_id, false)
            };
        let mut receiver_ty_id = receiver_ty_id;

        // keep import.meta receivers anchored on the language import-meta type
        if matches!(ctx.tree.get(receiver_id), Expression::ImportMeta)
            && let Some(import_meta_receiver_ty_id) = self
                .resolve_import_meta_receiver_type_for_infer(
                    &mut ctx.reborrow(),
                    receiver_id,
                    state,
                )?
        {
            receiver_ty_id = import_meta_receiver_ty_id;
        }

        // preserve unresolved infer vars only for direct binding references
        let preserve_infer_vars_for_lookup = matches!(
            ctx.tree.get(receiver_id),
            Expression::LocalReference { .. }
                | Expression::ModuleReference { .. }
                | Expression::GlobalReference { .. }
        );

        // materialize and normalize the receiver before lookup
        receiver_ty_id = self.normalize_member_receiver_type_for_lookup(
            &mut ctx.reborrow(),
            receiver_ty_id,
            preserve_infer_vars_for_lookup,
            state,
        )?;

        // recompute direct receiver references when cached receiver typing stayed unevaluated
        if matches!(ctx.types.get_type(receiver_ty_id), Type::Unevaluated(_))
            && let Some(recomputed_receiver_ty_id) = self.infer_member_receiver_type_without_cache(
                &mut ctx.reborrow(),
                receiver_id,
                state,
            )?
        {
            receiver_ty_id = self.normalize_member_receiver_type_for_lookup(
                &mut ctx.reborrow(),
                recomputed_receiver_ty_id,
                preserve_infer_vars_for_lookup,
                state,
            )?;
        }

        // query pre-existing receiver type commitments when receiver typing is still unevaluated
        if matches!(ctx.types.get_type(receiver_ty_id), Type::Unevaluated(_))
            && let Some(resolved_receiver_ty_id) =
                self.query_member_access_receiver_type_for_query(ctx, receiver_id)
        {
            receiver_ty_id = self.normalize_type_with_relation(
                &mut ctx.type_context_reborrow(),
                resolved_receiver_ty_id,
                NormalizationMode::Assign,
                RelationMode::ASSIGN,
            );
        }

        // resolve unevaluated receivers through canonical reference symbols
        if matches!(ctx.types.get_type(receiver_ty_id), Type::Unevaluated(_))
            && let Some(symbol_receiver_ty_id) = self.resolve_member_receiver_type_from_symbol(
                &mut ctx.reborrow(),
                receiver_id,
                state,
            )?
        {
            receiver_ty_id = self.normalize_member_receiver_type_for_lookup(
                &mut ctx.reborrow(),
                symbol_receiver_ty_id,
                preserve_infer_vars_for_lookup,
                state,
            )?;
        }

        // keep import.meta receivers anchored on the language import-meta instance
        if matches!(ctx.types.get_type(receiver_ty_id), Type::Unevaluated(_))
            && matches!(ctx.tree.get(receiver_id), Expression::ImportMeta)
            && let Some(import_meta_symbol) =
                self.get_language_symbol(ctx.profile, LanguageSymbol::ImportMeta)
            && let Some(import_meta_instance_ty_id) = self
                .apparent_instance_type(
                    &mut ctx.type_context_reborrow(),
                    receiver_id.into_any(),
                    import_meta_symbol,
                )
                .or(self.import_instance_type_for_symbol(
                    &ctx.index,
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    receiver_id.into_any(),
                    import_meta_symbol,
                    ctx.types,
                )?)
        {
            receiver_ty_id = self.normalize_type_with_relation(
                &mut ctx.type_context_reborrow(),
                import_meta_instance_ty_id,
                NormalizationMode::Assign,
                RelationMode::ASSIGN,
            );
        }

        // re-anchor local references through declared or inferred node commitments
        if matches!(ctx.types.get_type(receiver_ty_id), Type::Unevaluated(_))
            && let Some(receiver_symbol) =
                self.reference_symbol_for_expression(ctx.tree_symbol_view(), receiver_id)
        {
            let receiver_symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                receiver_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            if receiver_symbol.module_id == ctx.module.id
                && let Some(primary_declaration) = ctx
                    .symbols
                    .get_symbol(receiver_symbol.local_id)
                    .primary_declaration
                && let Some(declared_or_inferred_ty_id) = ctx
                    .types
                    .get_declared_or_inferred_type_id(primary_declaration)
            {
                let declared_or_inferred_ty_id = self.ensure_unwrapped_value_type_evaluated(
                    &mut ctx.reborrow(),
                    declared_or_inferred_ty_id,
                )?;
                receiver_ty_id = self.normalize_type_with_relation(
                    &mut ctx.type_context_reborrow(),
                    declared_or_inferred_ty_id,
                    NormalizationMode::Assign,
                    RelationMode::ASSIGN,
                );
            }
        }

        // record the lookup ready receiver type for query consumers
        ctx.types.set_member_receiver_type_for_node(
            receiver_id.into_global_any(ctx.module.id),
            receiver_ty_id,
        );

        let receiver_ty = ctx.types.get_type(receiver_ty_id).clone();
        // classify receiver semantics once for member lookup and diagnostic deferral
        let receiver_context = self.query_member_receiver_context_for_expression(
            &ctx.type_context_reborrow(),
            receiver_id,
            Some(receiver_ty_id),
        );

        // track whether member validation should wait for infer convergence
        let receiver_requires_infer_convergence = self.type_relation_requires_infer_convergence(
            ctx.type_view(),
            receiver_ty_id,
            receiver_ty_id,
        ) || (receiver_context.has_this_receiver
            && matches!(receiver_ty, Type::This));
        let allow_missing_member_deferral =
            self.context_function_uses_lambda_signature(state, ctx.tree);
        let force_unknown_receiver_diagnostic =
            self.context_function_is_non_lambda_property_method(state, ctx.tree);

        Ok(MemberAccessReceiverQuery::Receiver(MemberAccessReceiver {
            receiver_id,
            receiver_ty_id,
            receiver_ty,
            receiver_context,
            receiver_requires_infer_convergence,
            allow_missing_member_deferral,
            force_unknown_receiver_diagnostic,
            has_optional_nullish,
        }))
    }

    /// Query one receiver type id through the secondary receiver-resolution path.
    fn query_member_access_receiver_type_for_query(
        &self,
        ctx: &InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> Option<LocalTypeId> {
        let receiver_node = receiver_id.into_global_any(ctx.module.id);
        ctx.infer
            .inferred_type_for_node(receiver_node)
            .or_else(|| ctx.types.get_declared_or_inferred_type_id(receiver_node))
    }

    /// Materialize and normalize one member receiver type for lookup.
    fn normalize_member_receiver_type_for_lookup(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_ty_id: LocalTypeId,
        preserve_infer_vars_for_lookup: bool,
        _state: &InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let receiver_ty_id = if preserve_infer_vars_for_lookup {
            receiver_ty_id
        } else {
            self.materialize_infer_type_for_check(&mut ctx.reborrow(), receiver_ty_id)
        };
        let receiver_ty_id = self.normalize_type_with_relation(
            &mut ctx.type_context_reborrow(),
            receiver_ty_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let receiver_ty_id =
            self.ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), receiver_ty_id)?;
        let receiver_ty_id = self.normalize_type_with_relation(
            &mut ctx.type_context_reborrow(),
            receiver_ty_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );

        Ok(receiver_ty_id)
    }

    /// Resolve one unevaluated member receiver type through canonical symbol ownership.
    fn resolve_member_receiver_type_from_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        state: &InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // import.meta receivers resolve through the language import-meta symbol
        if matches!(ctx.tree.get(receiver_id), Expression::ImportMeta)
            && let Some(import_meta_symbol) =
                self.get_language_symbol(ctx.profile, LanguageSymbol::ImportMeta)
            && let Some(import_meta_receiver_ty_id) = self
                .require_instance_type(
                    &mut ctx.type_context_reborrow(),
                    receiver_id.into_any(),
                    import_meta_symbol,
                )
                .or(self.import_instance_type_for_symbol(
                    &ctx.index,
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    receiver_id.into_any(),
                    import_meta_symbol,
                    ctx.types,
                )?)
        {
            return Ok(Some(import_meta_receiver_ty_id));
        }

        if matches!(ctx.tree.get(receiver_id), Expression::ImportMeta)
            && let Some(import_meta_symbol) =
                self.get_language_symbol(ctx.profile, LanguageSymbol::ImportMeta)
            && import_meta_symbol.module_id != ctx.module.id
        {
            let import_meta_receiver_ty_id = self.resolve_remote_symbol_value_type_for_context(
                &mut ctx.reborrow(),
                state,
                receiver_id.into_any(),
                import_meta_symbol,
            )?;
            return Ok(Some(import_meta_receiver_ty_id));
        }

        // resolve direct reference receivers through canonical symbols
        let Some(receiver_symbol) =
            self.reference_symbol_for_expression(ctx.tree_symbol_view(), receiver_id)
        else {
            return Ok(None);
        };
        let receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // remote symbols use remote interface or surface value commitments
        if receiver_symbol.module_id != ctx.module.id {
            let receiver_type_id = self.resolve_remote_symbol_value_type_for_context(
                &mut ctx.reborrow(),
                state,
                receiver_id.into_any(),
                receiver_symbol,
            )?;
            return Ok(Some(receiver_type_id));
        }

        // local symbols prefer committed value types then declared or inferred node commitments
        if let Some(value_type_id) = ctx.types.get_value_type_id(receiver_symbol) {
            return Ok(Some(value_type_id));
        }

        let symbol_entry = ctx.symbols.get_symbol(receiver_symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(None);
        };
        Ok(ctx
            .types
            .get_declared_or_inferred_type_id(primary_declaration))
    }

    /// Resolve one query-facing member target from dynamic dispatch candidates.
    fn resolve_dynamic_member_target_symbol(
        &self,
        ctx: &InferContext<'_>,
        candidates: &[MemberResolutionCandidate],
    ) -> Option<GlobalSymbolId> {
        // collapse direct duplicates before following canonical identity
        let first_symbol = candidates.first()?.symbol;
        if candidates
            .iter()
            .all(|candidate| candidate.symbol == first_symbol)
        {
            return Some(first_symbol);
        }

        // only publish one canonical target when all dynamic candidates share it
        let first_canonical = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            first_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let all_same_canonical = candidates.iter().skip(1).all(|candidate| {
            self.canonical_symbol_id(
                ctx.module_symbol_view(),
                candidate.symbol,
                CanonicalSymbolMode::FollowAliases,
            ) == first_canonical
        });
        if all_same_canonical {
            return Some(first_canonical);
        }

        None
    }

    /// Resolve member lookup state for a prepared receiver.
    fn resolve_member_access_lookup(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver: &MemberAccessReceiver,
        member_key: StaticKey,
        state: &mut InferState,
    ) -> AnalyzeResult<MemberAccessLookup> {
        // ensure instance types are available for reference receivers
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            receiver.receiver_ty_id,
        )?;

        // inherit static arguments and substitutions from the receiver
        let mut inherited = self.resolve_inherited_static_arguments(
            &mut ctx.reborrow(),
            receiver.receiver_id.into_any(),
            Some(receiver.receiver_ty_id),
            &receiver.receiver_ty,
        )?;

        // classify receiver semantics once for all member lookup paths
        let receiver_context = receiver.receiver_context;

        // resolve member dispatch for the receiver type
        let resolution = self.resolve_member_symbol_for_receiver(
            &mut ctx.reborrow(),
            expression_id,
            receiver.receiver_id,
            &receiver.receiver_ty,
            &receiver_context,
            &member_key,
        )?;

        // reject implicit dynamic dispatch when configured
        if state.options.no_implicit_dynamic_dispatch
            && matches!(ctx.module.source, ModuleSource::User)
            && matches!(resolution, MemberResolution::Dynamic { .. })
        {
            self.error(AnalyzeError::ImplicitDynamicDispatchDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // resolve the selected member symbol for static dispatch paths
        let member_symbol = match &resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // enforce visibility for resolved members
        self.check_member_resolution_visibility(
            &mut ctx.reborrow(),
            expression_id,
            &resolution,
            member_symbol,
            receiver.receiver_ty_id,
            &member_key,
            state,
        )?;

        // resolve enum field symbols for enum value receivers
        let member_symbol = self.resolve_enum_field_member_symbol(
            &mut ctx.reborrow(),
            receiver.receiver_id,
            &member_key,
            member_symbol,
        )?;

        // publish the selected member target for query consumers
        let query_member_symbol = match &resolution {
            MemberResolution::Static { .. } => member_symbol,
            MemberResolution::Dynamic { candidates } => {
                self.resolve_dynamic_member_target_symbol(ctx, candidates)
            }
            _ => None,
        };
        if let Some(query_member_symbol) = query_member_symbol {
            let source_id = ctx.tree.get_source(expression_id.id);
            let span_type = Expression::member_source_part(ctx.tree, expression_id);
            ctx.types.set_symbol_target_for_source_part(
                SourcePartKey::new(source_id, span_type),
                query_member_symbol,
            );
        }

        if let Some(member_symbol) = member_symbol {
            self.extend_owner_substitutions_from_inherited(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                &mut inherited.substitutions,
            );
        }
        let enum_field_value_ty_id =
            self.enum_field_value_type_for_symbol(ctx.symbol_type_view(), member_symbol)?;

        // resolve extension substitutions for member symbols
        let extension_context = if let Some(member_symbol) = member_symbol {
            self.resolve_extension_member_context(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
            )?
        } else {
            None
        };

        // merge inherited and extension substitutions
        let substitutions = self.merge_member_substitutions(&inherited, extension_context.as_ref());

        Ok(MemberAccessLookup {
            member_key,
            receiver_context,
            resolution,
            member_symbol,
            enum_field_value_ty_id,
            inherited,
            extension_context,
            substitutions,
        })
    }

    /// Query one member type from the receiver shape when its dependencies are ready.
    fn query_member_type_from_receiver_shape(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        member_symbol: Option<GlobalSymbolId>,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // let concrete symbol lookup drive static associated member typing
        if member_symbol.is_some() {
            return match self.infer_member_of_type(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                receiver_ty,
                member_key,
                lookup_mode,
                visited,
            ) {
                Ok(member_ty_id) => Ok(member_ty_id),
                Err(AnalyzeError::Yield { .. }) => Ok(None),
                Err(error) => Err(error),
            };
        }

        let member_ty_id = self.infer_member_of_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            receiver_ty,
            member_key,
            lookup_mode,
            visited,
        )?;

        Ok(member_ty_id)
    }

    /// Infer and commit the member access type from resolved lookup state.
    fn infer_member_access_type_from_lookup(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        static_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        receiver: &MemberAccessReceiver,
        lookup: &MemberAccessLookup,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the member type from the receiver shape
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.query_member_type_from_receiver_shape(
            &mut ctx.reborrow(),
            expression_id,
            &receiver.receiver_ty,
            &lookup.member_key,
            lookup.receiver_context.lookup_mode,
            lookup.member_symbol,
            &mut member_type_visited,
        )?;
        let member_ty_id = self.resolve_member_type_for_symbol(
            &mut ctx.reborrow(),
            expression_id,
            lookup.member_symbol,
            member_ty_id,
        )?;

        // project through the resolved nominal receiver, not just the original lookup context
        let projection_receiver = self.unwrap_type_symbol(ctx.types, receiver.receiver_ty_id);
        let (projection_receiver_symbol, projection_receiver_arguments) =
            if let Some((symbol, arguments, _)) = projection_receiver {
                let arguments = arguments.unwrap_or_else(|| lookup.inherited.arguments.clone());
                let (symbol, arguments) = self.normalize_projection_receiver_reference(
                    &mut ctx.type_context_reborrow(),
                    expression_id.into_any(),
                    symbol,
                    &arguments,
                )?;
                (Some(symbol), arguments)
            } else if let Some(symbol) = lookup.receiver_context.nominal_symbol {
                let arguments = lookup.inherited.arguments.clone();
                let (symbol, arguments) = self.normalize_projection_receiver_reference(
                    &mut ctx.type_context_reborrow(),
                    expression_id.into_any(),
                    symbol,
                    &arguments,
                )?;
                (Some(symbol), arguments)
            } else {
                (None, lookup.inherited.arguments.clone())
            };

        // resolve member type through symbol lookup and remaining lookup paths
        let mut resolved_member_ty_id = if let Some(enum_field_value_ty_id) =
            lookup.enum_field_value_ty_id
        {
            enum_field_value_ty_id
        } else if let Some(member_ty_id) = member_ty_id {
            let resolved_member = self.resolve_member_access_type_for_symbol(
                &mut ctx.reborrow(),
                expression_id,
                lookup.member_symbol,
                projection_receiver_symbol,
                &projection_receiver_arguments,
                member_ty_id,
                static_arguments,
                &lookup.substitutions,
            )?;
            let member_instance_id = if static_arguments.is_some() {
                if let Some(member_symbol) = lookup.member_symbol {
                    self.record_member_instance_for_arguments(
                        &mut ctx.reborrow(),
                        expression_id,
                        member_symbol,
                        &lookup.inherited,
                        lookup.extension_context.as_ref(),
                        &resolved_member.static_arguments,
                        &resolved_member.static_parameter_symbols,
                    )?
                } else {
                    None
                }
            } else {
                None
            };

            // record resolved member access for downstream passes
            self.record_provisional_member_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver.receiver_ty_id),
                &lookup.resolution,
                member_instance_id,
                None,
                true,
                &mut *ctx.infer,
                &mut *ctx.types,
            );
            resolved_member.type_id
        } else {
            self.resolve_member_index_or_missing(
                &mut ctx.reborrow(),
                super::resolve::MissingMemberResolutionContext {
                    expression_id,
                    receiver_id: receiver.receiver_id,
                    receiver_ty_id: receiver.receiver_ty_id,
                    receiver_ty: &receiver.receiver_ty,
                    receiver_requires_infer_convergence: receiver
                        .receiver_requires_infer_convergence,
                    allow_missing_member_deferral: receiver.allow_missing_member_deferral,
                    force_unknown_receiver_diagnostic: receiver.force_unknown_receiver_diagnostic,
                    member_key: &lookup.member_key,
                    member_resolution: &lookup.resolution,
                    is_surface_inference: state.is_surface_inference,
                },
            )?
        };

        // substitute `this` in member result types with the resolved receiver type
        resolved_member_ty_id = {
            let mut cache = HashMap::new();
            self.substitute_this_type(
                resolved_member_ty_id,
                receiver.receiver_ty_id,
                &mut *ctx.types,
                &mut cache,
            )
        };
        resolved_member_ty_id = self.materialize_associated_member_access_type(
            &mut ctx.reborrow(),
            expression_id,
            resolved_member_ty_id,
            lookup,
            projection_receiver_symbol,
            &projection_receiver_arguments,
        )?;

        // register associated comptime obligations until post infer convergence
        let obligation_member_symbol = self.projection_member_symbol_for_expression(
            &mut ctx.reborrow(),
            expression_id,
            receiver.receiver_id,
            &lookup.member_key,
            lookup.member_symbol,
        )?;
        let receiver_is_projection_receiver =
            self.is_projection_receiver_expression(&mut ctx.reborrow(), receiver.receiver_id);
        let member_type_is_unevaluated = matches!(
            ctx.types.get_type(resolved_member_ty_id),
            Type::Unevaluated(_)
        );
        let requires_projection_obligation = if let Some(member_symbol) = obligation_member_symbol {
            self.projection_requires_associated_comptime_obligation(
                ctx.tree_symbol_view(),
                member_symbol,
                lookup.receiver_context.has_static_arguments,
            )?
        } else {
            (lookup.receiver_context.has_static_arguments || member_type_is_unevaluated)
                && receiver_is_projection_receiver
        };
        if requires_projection_obligation {
            ctx.infer.push_associated_comptime_projection_obligation(
                AssociatedComptimeProjectionObligation {
                    expression_id,
                    member_symbol: obligation_member_symbol,
                    member_type_id: resolved_member_ty_id,
                    receiver_arguments: projection_receiver_arguments.clone(),
                    substitutions: lookup.substitutions.clone(),
                },
            );
        }

        Ok(resolved_member_ty_id)
    }

    /// Resolve one associated-comptime member symbol for deferred projection obligations.
    fn projection_member_symbol_for_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
        resolved_member_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        if resolved_member_symbol.is_some() {
            return Ok(resolved_member_symbol);
        }

        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, ctx.tree);
        let is_projection_receiver =
            self.is_projection_receiver_expression(&mut ctx.reborrow(), receiver_id);
        if !is_projection_receiver {
            return Ok(None);
        }

        let selection = self.select_associated_projection_member_symbol(
            &mut ctx.type_context_reborrow(),
            expression_id,
            receiver_id,
            *member_key,
            Some(StaticMemberSymbolKind::AssociatedComptimeConst),
            true,
            true,
        )?;

        Ok(selection.map(|selection| selection.target_symbol))
    }

    /// Materialize one associated comptime member access after receiver substitution.
    fn materialize_associated_member_access_type(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_ty_id: LocalTypeId,
        lookup: &MemberAccessLookup,
        projection_receiver_symbol: Option<GlobalSymbolId>,
        projection_receiver_arguments: &[StaticArgument],
    ) -> AnalyzeResult<LocalTypeId> {
        let Some(member_symbol) = lookup.member_symbol else {
            return Ok(member_ty_id);
        };

        let kind =
            self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), member_symbol)?;
        if kind != Some(StaticMemberSymbolKind::AssociatedComptimeConst) {
            return Ok(member_ty_id);
        }

        // defer projection materialization until receiver static arguments converge
        // unresolved obligations are reported after infer convergence
        if self.receiver_projection_arguments_require_deferral(
            ctx.type_view(),
            &lookup.inherited.arguments,
        ) {
            return Ok(member_ty_id);
        }

        let member_ty = ctx.types.get_type(member_ty_id).clone();
        let materialized_ty = self.materialize_associated_member_projection(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            member_symbol,
            projection_receiver_symbol,
            projection_receiver_arguments,
            None,
            member_ty,
        )?;

        Ok(ctx
            .types
            .insert_type_from_type(materialized_ty, member_ty_id))
    }
    /// Return the member access type for `any` receivers.
    pub(crate) fn any_member_access_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // preserve any when the receiver is any
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Any,
        };
        types.insert_type_from(ty, expression_id)
    }
}
