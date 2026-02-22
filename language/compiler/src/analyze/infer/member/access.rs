use super::*;
use destack_dir::{FunctionKind, Property};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a member access expression.
    pub(crate) fn infer_member_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        member_name: StringId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_MEMBER);

        // query and normalize the receiver state
        let receiver = match self.query_member_access_receiver(
            module,
            expression_id,
            left_id,
            tree,
            symbols,
            types,
            infer,
            ctx,
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
            let type_id = self.any_member_access_type(expression_id, types);
            return Ok(finish_result(type_id, types));
        }

        // resolve the lookup key from member syntax
        let member_key = StaticKey::Name(member_name);

        // handle enum field access early to preserve nominal enum types
        if let Some(enum_reference_id) = self.resolve_enum_field_access(
            module,
            expression_id,
            receiver.receiver_id,
            receiver.receiver_ty_id,
            &receiver.receiver_ty,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
        )? {
            return Ok(finish_result(enum_reference_id, types));
        }

        // resolve member symbol and substitution state for this receiver
        let lookup = self.resolve_member_access_lookup(
            module,
            expression_id,
            &receiver,
            member_key,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // infer and commit the member access type from resolved lookup state
        let resolved_member_ty_id = self.infer_member_access_type_from_lookup(
            module,
            expression_id,
            static_arguments,
            &receiver,
            &lookup,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        Ok(finish_result(resolved_member_ty_id, types))
    }

    /// Return true when the active function context uses lambda signature semantics.
    fn context_function_uses_lambda_signature(&self, ctx: &InferContext, tree: &NodeTree) -> bool {
        let Some(function_id) = ctx.in_function else {
            return false;
        };

        match function_id.ty {
            NodeType::Declaration => {
                let declaration = tree.get(function_id.into_typed::<Declaration>());
                let Declaration::Function { signature, .. } = declaration else {
                    return false;
                };

                signature.kind == FunctionKind::Lambda
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

    /// Query and normalize the receiver state for member access.
    #[allow(clippy::too_many_arguments)]
    fn query_member_access_receiver(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<MemberAccessReceiverQuery> {
        // optional chain receivers must unwrap maybe before member lookup
        let optional_chain =
            self.infer_optional_chain_receiver(module, left_id, tree, symbols, types, infer, ctx)?;
        let (receiver_id, receiver_ty_id, has_optional_nullish) =
            if let Some(optional_chain) = optional_chain {
                let Some(receiver_ty_id) = optional_chain.receiver_ty_id else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    };
                    let undefined_ty_id = types.insert_type_from(ty, expression_id);
                    return Ok(MemberAccessReceiverQuery::EarlyType(undefined_ty_id));
                };
                (
                    optional_chain.receiver_id,
                    receiver_ty_id,
                    optional_chain.has_nullish,
                )
            } else {
                let receiver_ty_id = if self.query_expression_is_projection_receiver_for_infer(
                    module,
                    ctx.profile,
                    left_id,
                    tree,
                    symbols,
                    types,
                ) {
                    self.resolve_declared_type_expression(
                        module,
                        ctx.profile,
                        left_id,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )?
                } else {
                    self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?
                };
                (left_id, receiver_ty_id, false)
            };

        // materialize and normalize the receiver before lookup
        let receiver_ty_id = self.materialize_infer_type_for_check(
            module,
            ctx.profile,
            symbols,
            receiver_ty_id,
            infer,
            types,
            &ctx.options,
        );
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

        // classify receiver semantics once for member lookup and diagnostic deferral
        let receiver_context = self.query_member_receiver_context_for_expression(
            module,
            receiver_id,
            Some(receiver_ty_id),
            &receiver_ty,
            ctx.profile,
            tree,
            symbols,
            types,
        );

        // track whether member validation should wait for infer convergence
        let receiver_requires_infer_convergence = self.type_relation_requires_infer_convergence(
            module,
            ctx.profile,
            receiver_ty_id,
            receiver_ty_id,
            symbols,
            types,
        ) || (receiver_context.has_this_receiver
            && matches!(receiver_ty, Type::This));
        let allow_missing_member_deferral = self.context_function_uses_lambda_signature(ctx, tree);

        Ok(MemberAccessReceiverQuery::Receiver(MemberAccessReceiver {
            receiver_id,
            receiver_ty_id,
            receiver_ty,
            receiver_context,
            receiver_requires_infer_convergence,
            allow_missing_member_deferral,
            has_optional_nullish,
        }))
    }

    /// Resolve member lookup state for a prepared receiver.
    #[allow(clippy::too_many_arguments)]
    fn resolve_member_access_lookup(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver: &MemberAccessReceiver,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<MemberAccessLookup> {
        // ensure instance types are available for reference receivers
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            receiver.receiver_ty_id,
            types,
        )?;

        // inherit static arguments and substitutions from the receiver
        let mut inherited = self.resolve_inherited_static_arguments(
            module,
            ctx.profile,
            receiver.receiver_id.into_any(),
            Some(receiver.receiver_ty_id),
            &receiver.receiver_ty,
            infer,
            &ctx.options,
            tree,
            symbols,
            types,
        )?;

        // classify receiver semantics once for all member lookup paths
        let receiver_context = receiver.receiver_context.clone();

        // resolve member dispatch for the receiver type
        let resolution = self.resolve_member_symbol_for_receiver(
            module,
            receiver.receiver_id,
            &receiver.receiver_ty,
            &receiver_context,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
        )?;

        // reject implicit dynamic dispatch when configured
        if ctx.options.no_implicit_dynamic_dispatch
            && matches!(module.source, ModuleSource::User)
            && matches!(resolution, MemberResolution::Dynamic { .. })
        {
            self.error(AnalyzeError::ImplicitDynamicDispatchDisabled {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // resolve static member symbol from dispatch mode
        let member_symbol = match &resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // enforce visibility for resolved members
        self.check_member_resolution_visibility(
            module,
            expression_id,
            &resolution,
            member_symbol,
            receiver.receiver_ty_id,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
            ctx,
        )?;

        // resolve enum field symbols for enum value receivers
        let member_symbol = self.resolve_enum_field_member_symbol(
            module,
            receiver.receiver_id,
            &member_key,
            member_symbol,
            ctx.profile,
            tree,
            symbols,
            types,
        )?;
        if let Some(member_symbol) = member_symbol {
            self.extend_owner_substitutions_from_inherited_arguments(
                module,
                ctx.profile,
                expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                &mut inherited.substitutions,
                tree,
                symbols,
                types,
            );
        }
        let enum_field_value_ty_id = self.enum_field_value_type_for_symbol(
            module,
            ctx.profile,
            symbols,
            member_symbol,
            types,
        )?;

        // resolve extension substitutions for member symbols
        let extension_context = if let Some(member_symbol) = member_symbol {
            self.resolve_extension_member_context(
                module,
                ctx.profile,
                expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                &ctx.options,
                tree,
                symbols,
                types,
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

    /// Infer and commit the member access type from resolved lookup state.
    #[allow(clippy::too_many_arguments)]
    fn infer_member_access_type_from_lookup(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        receiver: &MemberAccessReceiver,
        lookup: &MemberAccessLookup,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the member type from the receiver shape
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            symbols,
            &receiver.receiver_ty,
            &lookup.member_key,
            lookup.receiver_context.lookup_mode,
            types,
            &mut member_type_visited,
        )?;
        let member_ty_id = self.resolve_member_type_for_symbol(
            module,
            expression_id,
            lookup.member_symbol,
            member_ty_id,
            ctx.profile,
            types,
        )?;

        // resolve member type through symbol lookup and remaining lookup paths
        let mut resolved_member_ty_id =
            if let Some(enum_field_value_ty_id) = lookup.enum_field_value_ty_id {
                enum_field_value_ty_id
            } else if let Some(member_ty_id) = member_ty_id {
                let resolved_member = self.resolve_member_access_type_for_symbol(
                    module,
                    expression_id,
                    lookup.member_symbol,
                    member_ty_id,
                    static_arguments,
                    &lookup.substitutions,
                    ctx.profile,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                let member_instance_id = if static_arguments.is_some() {
                    if let Some(member_symbol) = lookup.member_symbol {
                        self.commit_member_instance_for_arguments(
                            module,
                            ctx.profile,
                            expression_id,
                            member_symbol,
                            &lookup.inherited,
                            lookup.extension_context.as_ref(),
                            &resolved_member.static_arguments,
                            &resolved_member.static_parameter_symbols,
                            tree,
                            symbols,
                            infer,
                            types,
                        )?
                    } else {
                        None
                    }
                } else {
                    None
                };

                // record resolved member access for downstream passes
                self.commit_member_resolution(
                    expression_id.into_global_any(module.id),
                    Some(receiver.receiver_ty_id),
                    &lookup.resolution,
                    member_instance_id,
                    None,
                    true,
                    types,
                );
                resolved_member.type_id
            } else {
                self.resolve_member_index_or_missing(
                    module,
                    expression_id,
                    receiver.receiver_id,
                    receiver.receiver_ty_id,
                    &receiver.receiver_ty,
                    receiver.receiver_requires_infer_convergence,
                    receiver.allow_missing_member_deferral,
                    &lookup.member_key,
                    &lookup.resolution,
                    ctx.profile,
                    ctx.is_surface_inference,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?
            };

        // substitute `this` in member result types with the resolved receiver type
        resolved_member_ty_id = {
            let mut cache = HashMap::new();
            self.substitute_this_type(
                resolved_member_ty_id,
                receiver.receiver_ty_id,
                types,
                &mut cache,
            )
        };
        resolved_member_ty_id = self.materialize_associated_comptime_member_access_type(
            module,
            expression_id,
            resolved_member_ty_id,
            lookup,
            ctx.profile,
            tree,
            symbols,
            types,
        )?;

        // register associated comptime obligations until post infer convergence
        let obligation_member_symbol = self.projection_obligation_member_symbol_for_expression(
            module,
            ctx.profile,
            expression_id,
            receiver.receiver_id,
            &lookup.member_key,
            lookup.member_symbol,
            tree,
            symbols,
            types,
        )?;
        let receiver_is_projection_receiver = self
            .query_expression_is_projection_receiver_for_infer(
                module,
                ctx.profile,
                receiver.receiver_id,
                tree,
                symbols,
                types,
            );
        let member_type_is_unevaluated =
            matches!(types.get_type(resolved_member_ty_id), Type::Unevaluated(_));
        let requires_projection_obligation = if let Some(member_symbol) = obligation_member_symbol {
            self.projection_requires_associated_comptime_obligation(
                module,
                ctx.profile,
                member_symbol,
                lookup.receiver_context.has_static_arguments,
                tree,
                symbols,
            )?
        } else {
            (lookup.receiver_context.has_static_arguments || member_type_is_unevaluated)
                && receiver_is_projection_receiver
        };
        if requires_projection_obligation {
            infer.push_associated_comptime_projection_obligation(
                AssociatedComptimeProjectionObligation {
                    expression_id,
                    member_symbol: obligation_member_symbol,
                    member_type_id: resolved_member_ty_id,
                    receiver_arguments: lookup.inherited.arguments.clone(),
                    substitutions: lookup.substitutions.clone(),
                },
            );
        }

        Ok(resolved_member_ty_id)
    }

    /// Resolve one associated-comptime member symbol for deferred projection obligations.
    #[allow(clippy::too_many_arguments)]
    fn projection_obligation_member_symbol_for_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
        resolved_member_symbol: Option<GlobalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        if resolved_member_symbol.is_some() {
            return Ok(resolved_member_symbol);
        }

        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);
        if !self.query_expression_is_projection_receiver_for_infer(
            module,
            profile,
            receiver_id,
            tree,
            symbols,
            types,
        ) {
            return Ok(None);
        }

        let selection = self.select_associated_projection_member_symbol(
            module,
            profile,
            expression_id,
            receiver_id,
            *member_key,
            Some(StaticMemberSymbolKind::AssociatedComptimeConst),
            tree,
            symbols,
            types,
            true,
            true,
        )?;

        Ok(selection.map(|selection| selection.target_symbol))
    }

    /// Materialize one associated comptime member access after receiver substitution.
    #[allow(clippy::too_many_arguments)]
    fn materialize_associated_comptime_member_access_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_ty_id: LocalTypeId,
        lookup: &MemberAccessLookup,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let Some(member_symbol) = lookup.member_symbol else {
            return Ok(member_ty_id);
        };

        let kind = self.query_static_member_symbol_kind_for_symbol(
            module,
            profile,
            member_symbol,
            tree,
            symbols,
        )?;
        if kind != Some(crate::analyze::StaticMemberSymbolKind::AssociatedComptimeConst) {
            return Ok(member_ty_id);
        }

        // defer projection materialization until receiver static arguments converge
        // unresolved obligations are reported after infer convergence
        if self.associated_projection_receiver_arguments_require_deferral(
            module,
            profile,
            &lookup.inherited.arguments,
            symbols,
            types,
        ) {
            return Ok(member_ty_id);
        }

        let member_ty = types.get_type(member_ty_id).clone();
        let materialized_ty = self.materialize_associated_member_projection(
            module,
            profile,
            expression_id.into_any(),
            member_symbol,
            lookup.receiver_context.nominal_symbol,
            &lookup.inherited.arguments,
            None,
            member_ty,
            tree,
            symbols,
            types,
        )?;

        Ok(types.insert_type_from_type(materialized_ty, member_ty_id))
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
