use super::*;
use crate::CompilerContext;
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{
    AnalyzeIndex, CanonicalSymbolMode, InferContext, ModuleSymbolView, ModuleTypeView,
    SymbolTypeView, TypeContext, TypeView,
};
use crate::analyze::infer::RemoteValueTypeReadDomain;
use crate::analyze::module::GlobalMergeCategory;
use destack_dir::{Name, SymbolSpaceOrder, WellKnownSymbol};

/// Inputs for resolving member index-signature fallback versus missing-member diagnostics.
#[derive(Clone, Copy)]
pub(crate) struct MissingMemberResolutionContext<'a> {
    /// The member access expression id.
    pub(crate) expression_id: LocalNodeId<Expression>,
    /// The member receiver expression id.
    pub(crate) receiver_id: LocalNodeId<Expression>,
    /// The inferred receiver type id.
    pub(crate) receiver_ty_id: LocalTypeId,
    /// The inferred receiver type value.
    pub(crate) receiver_ty: &'a Type,
    /// Whether receiver typing still depends on infer convergence.
    pub(crate) receiver_requires_infer_convergence: bool,
    /// Whether missing-member diagnostics may be deferred.
    pub(crate) allow_missing_member_deferral: bool,
    /// Whether indeterminate receiver member checks should report unknown diagnostics immediately.
    pub(crate) force_unknown_receiver_diagnostic: bool,
    /// The member key used for lookup.
    pub(crate) member_key: &'a StaticKey,
    /// The resolved member dispatch state.
    pub(crate) member_resolution: &'a MemberResolution,
    /// Whether this is surface inference.
    pub(crate) is_surface_inference: bool,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve one canonical carrier symbol for implicit well-known member lookup.
    fn resolve_implicit_well_known_carrier_symbol(
        &self,
        profile: ProfileId,
        well_known_symbol: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        if let Some(symbol) = self.get_well_known_type_symbol(profile, well_known_symbol) {
            return Some(symbol);
        }

        if let Some(symbol) = self.get_well_known_symbol_from(
            profile,
            well_known_symbol,
            SymbolSpaceOrder::TypeThenValue,
        ) {
            return Some(symbol);
        }

        let symbol_name = self
            .repository
            .strings
            .intern(well_known_symbol.export_name());
        let symbol_key = StaticKey::Name(symbol_name);
        if let Some(symbol) = self
            .get_library_symbol_sources_for_space_order(
                profile,
                symbol_key,
                SymbolSpaceOrder::TypeThenValue,
            )
            .and_then(|sources| sources.into_iter().next())
        {
            return Some(symbol);
        }

        self.get_declared_library_symbol_from(profile, symbol_name, SymbolSpaceOrder::TypeThenValue)
    }

    /// Resolve the preferred member type for a symbol-aware lookup.
    pub(crate) fn resolve_member_type_for_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        inferred_member_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(member_symbol) = member_symbol else {
            return Ok(inferred_member_ty_id);
        };
        let member_symbol = self
            .normalize_member_symbol_for_declare_reads(ctx.module_symbol_view(), member_symbol)?;

        let mut member_ty_id = match (
            ctx.types.get_value_type_id(member_symbol),
            inferred_member_ty_id,
        ) {
            (Some(value_ty_id), Some(inferred_member_ty_id)) => {
                if self.is_infer_var_type(value_ty_id, ctx.types) {
                    Some(inferred_member_ty_id)
                } else {
                    Some(value_ty_id)
                }
            }
            (Some(value_ty_id), None) => Some(value_ty_id),
            (None, Some(inferred_member_ty_id)) => Some(inferred_member_ty_id),
            (None, None) => None,
        };

        // fall back to declaration-backed local member types when instance surfaces
        // have not materialized a value type yet
        if member_ty_id.is_none() && member_symbol.module_id == ctx.module.id {
            member_ty_id = self.local_member_type_for_symbol(
                &mut ctx.reborrow(),
                expression_id,
                member_symbol,
            )?;
        }

        // import remote member types when local ctx have no value type yet
        if member_ty_id.is_none() && member_symbol.module_id != ctx.module.id {
            let member_kind = self.query_static_member_symbol_kind_for_symbol(
                ctx.tree_symbol_view(),
                member_symbol,
            )?;
            if member_kind == Some(StaticMemberSymbolKind::AssociatedComptimeConst) {
                let associated_type_id = self.query_associated_member_type_for_symbol(
                    &mut ctx.reborrow(),
                    expression_id,
                    member_symbol,
                )?;
                member_ty_id = associated_type_id.or(member_ty_id);
                return Ok(member_ty_id);
            }

            let remote_ty_id = self.resolve_remote_symbol_value_type(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                member_symbol,
                RemoteValueTypeReadDomain::Interface,
            )?;
            member_ty_id = Some(remote_ty_id);
        }

        Ok(member_ty_id)
    }

    /// Resolve one declaration-backed local member type when no value type is available yet.
    fn local_member_type_for_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let symbol_entry = ctx.symbols.get_symbol(member_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration;

        if let Some(primary_declaration) = primary_declaration
            && let Some(signature_ty_id) =
                ctx.types.get_signature_type_for_node(primary_declaration)
        {
            return Ok(Some(signature_ty_id));
        }

        if let Some(primary_declaration) = primary_declaration
            && let Some(declared_ty_id) = ctx.types.get_declared_type_id(primary_declaration)
        {
            return Ok(Some(declared_ty_id));
        }

        let member_kind =
            self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), member_symbol)?;
        if member_kind == Some(StaticMemberSymbolKind::AssociatedComptimeConst) {
            return self.query_associated_member_type_for_symbol(
                &mut ctx.reborrow(),
                expression_id,
                member_symbol,
            );
        }

        Ok(None)
    }

    /// Normalize one member symbol to a declaration-backed symbol for declare reads.
    fn normalize_member_symbol_for_declare_reads(
        &self,
        view: ModuleSymbolView<'_>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<GlobalSymbolId> {
        let mut current_symbol =
            self.canonical_symbol_id(view, member_symbol, CanonicalSymbolMode::FollowAliases);
        let mut visited_symbols = HashSet::new();

        loop {
            if !visited_symbols.insert(current_symbol) {
                return Ok(current_symbol);
            }

            let (normalized_symbol, has_concrete_primary_declaration, next_symbol) = self
                .with_module_symbols_or_local_for_artifact(
                    view.compiler_context,
                    view.module,
                    view.profile,
                    current_symbol.module_id,
                    view.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |owner_module, owner_symbols| {
                        let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                        (
                            GlobalSymbolId::new(
                                owner_module.id,
                                current_symbol.local_id.with_type(symbol_entry.ty),
                            ),
                            symbol_entry.primary_declaration.is_some_and(|declaration| {
                                !matches!(
                                    declaration.local_id.ty,
                                    NodeType::DependencyItem | NodeType::Expression
                                )
                            }),
                            symbol_entry.target_symbol.or(symbol_entry.canonical_symbol),
                        )
                    },
                )
                .map_err(AnalyzeError::from)?;

            if has_concrete_primary_declaration {
                return Ok(normalized_symbol);
            }

            let Some(next_symbol) = next_symbol else {
                return Ok(normalized_symbol);
            };
            current_symbol =
                self.canonical_symbol_id(view, next_symbol, CanonicalSymbolMode::FollowAliases);
        }
    }

    /// Resolve one associated comptime member type without generic remote value import.
    fn query_associated_member_type_for_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        if member_symbol.module_id == ctx.module.id {
            if let Some(value_type_id) = ctx.types.get_value_type_id(member_symbol) {
                return Ok(Some(value_type_id));
            }

            let symbol_entry = ctx.symbols.get_symbol(member_symbol.local_id);
            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                return Ok(None);
            };
            if let Some(signature_type_id) =
                ctx.types.get_signature_type_for_node(primary_declaration)
            {
                return Ok(Some(signature_type_id));
            }
            if let Some(declared_type_id) = ctx.types.get_declared_type_id(primary_declaration) {
                return Ok(Some(declared_type_id));
            }

            if primary_declaration.local_id.ty == NodeType::Member {
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                if let Member::AssociatedConst {
                    declared_type: Some(member_type),
                    ..
                } = ctx.tree.get(member_id)
                {
                    if let Some(declared_type_id) = ctx.types.get_declared_type_id(
                        member_type.into_global_any(primary_declaration.module_id),
                    ) {
                        return Ok(Some(declared_type_id));
                    }

                    let declared_type_id = self.resolve_declared_type_expression(
                        &mut ctx.type_context_reborrow(),
                        *member_type,
                        true,
                        true,
                    )?;
                    return Ok(Some(declared_type_id));
                }
            }
            return Ok(None);
        }

        let remote_dir = self
            .require_artifact_dir_analyzed(
                ctx.compiler_context.revision(),
                member_symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        let mut remote_snapshot = remote_dir.types.as_ref().clone();
        let remote_symbol_entry = remote_dir.symbols.get_symbol(member_symbol.local_id);
        let resolved_symbol = GlobalSymbolId::new(
            member_symbol.module_id,
            member_symbol.local_id.with_type(remote_symbol_entry.ty),
        );

        let mut remote_type_id = remote_snapshot.get_value_type_id(resolved_symbol);
        if remote_type_id.is_none()
            && let Some(primary_declaration) = remote_symbol_entry.primary_declaration
        {
            remote_type_id = remote_snapshot.get_signature_type_for_node(primary_declaration);
        }
        if remote_type_id.is_none()
            && let Some(primary_declaration) = remote_symbol_entry.primary_declaration
        {
            remote_type_id = remote_snapshot.get_declared_type_id(primary_declaration);
        }
        if remote_type_id.is_none()
            && let Some(primary_declaration) = remote_symbol_entry.primary_declaration
            && primary_declaration.local_id.ty == NodeType::Member
        {
            let member_id = primary_declaration.local_id.into_typed::<Member>();
            if let Member::AssociatedConst {
                declared_type: Some(member_type),
                ..
            } = remote_dir.tree.get(member_id)
            {
                remote_type_id = remote_snapshot.get_declared_type_id(
                    member_type.into_global_any(primary_declaration.module_id),
                );
                if remote_type_id.is_none() {
                    let remote_options = ctx
                        .compiler_context
                        .analyze_context_options_for_module(member_symbol.module_id);
                    let remote_module = ctx.compiler_context.module(member_symbol.module_id);
                    let remote_module = remote_module.as_ref();
                    let mut view = TypeContext::new(
                        ctx.compiler_context,
                        remote_module,
                        ctx.profile,
                        &remote_options,
                        &remote_dir.tree,
                        &remote_dir.symbols,
                        &mut remote_snapshot,
                        ctx.index.clone(),
                    );
                    let evaluated_type_id =
                        self.resolve_declared_type_expression(&mut view, *member_type, true, true)?;
                    remote_type_id = Some(evaluated_type_id);
                }
            }
        }

        let remote_import = remote_type_id.map(|remote_type_id| {
            let remote_type = remote_snapshot.get_type(remote_type_id).clone();
            (resolved_symbol, remote_type, remote_snapshot)
        });

        let Some((_resolved_symbol, remote_type, remote_snapshot)) = remote_import else {
            return Ok(None);
        };

        let local_type_id = self.import_remote_type_for_node(
            expression_id.into_any(),
            &remote_type,
            &remote_snapshot,
            ctx.types,
        );
        Ok(Some(local_type_id))
    }

    /// Resolve member symbols for a receiver type when nominal dispatch is possible.
    pub(crate) fn resolve_member_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_ty: &Type,
        member_key: &StaticKey,
    ) -> AnalyzeResult<MemberResolution> {
        let resolution = match receiver_ty {
            Type::Reference { .. } => {
                // resolve nominal members first
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.module.id,
                    ctx.profile,
                    &ctx.index,
                    ctx.tree,
                    ctx.symbols,
                    &*ctx.types,
                    receiver_ty,
                    member_key,
                    &mut visited,
                    true,
                )?;
                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::Unresolved
                }
            }
            Type::TypeLiteral { .. } => {
                // resolve implicit members for primitive and literal receivers
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.module.id,
                    ctx.profile,
                    &ctx.index,
                    ctx.tree,
                    ctx.symbols,
                    &*ctx.types,
                    receiver_ty,
                    member_key,
                    &mut visited,
                    true,
                )?;
                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::Unresolved
                }
            }
            Type::Union { elements } => {
                // resolve member symbols for each union element
                let mut candidates = Vec::new();
                for element_id in elements {
                    let element_ty = ctx.types.get_type(*element_id).clone();
                    let mut visited = Vec::new();
                    let mut member_symbol = self.resolve_member_symbol_for_type(
                        ctx.compiler_context,
                        ctx.module,
                        ctx.module.id,
                        ctx.profile,
                        &ctx.index,
                        ctx.tree,
                        ctx.symbols,
                        &*ctx.types,
                        &element_ty,
                        member_key,
                        &mut visited,
                        true,
                    )?;
                    if member_symbol.is_none() {
                        // fall back to instance type owners when possible
                        if let Some(instance_symbol) =
                            ctx.types.symbol_for_instance_type(*element_id)
                        {
                            member_symbol = self.resolve_member_symbol_for_symbol(
                                ctx.compiler_context,
                                ctx.module,
                                ctx.module.id,
                                ctx.profile,
                                &ctx.index,
                                ctx.tree,
                                ctx.symbols,
                                &*ctx.types,
                                instance_symbol,
                                member_key,
                                MemberLookupMode::Instance,
                                &mut visited,
                            )?;
                        }
                    }
                    let Some(member_symbol) = member_symbol else {
                        return Ok(MemberResolution::None);
                    };
                    candidates.push(MemberResolutionCandidate {
                        receiver_ty_id: *element_id,
                        symbol: member_symbol,
                    });
                }

                // map resolution type depending on variants
                match candidates.len() {
                    0 => MemberResolution::None,
                    1 => MemberResolution::Static {
                        symbol: candidates[0].symbol,
                    },
                    _ => MemberResolution::Dynamic { candidates },
                }
            }
            _ => {
                // resolve implicit well known member resolution
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.module.id,
                    ctx.profile,
                    &ctx.index,
                    ctx.tree,
                    ctx.symbols,
                    &*ctx.types,
                    receiver_ty,
                    member_key,
                    &mut visited,
                    true,
                )?;
                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::None
                }
            }
        };

        Ok(resolution)
    }

    /// Resolve member symbols using the receiver expression when available.
    pub(crate) fn resolve_member_symbol_for_receiver(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        receiver_context: &MemberReceiverContext,
        member_key: &StaticKey,
    ) -> AnalyzeResult<MemberResolution> {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, ctx.tree);

        // resolve namespace import member paths before receiver-type dispatch
        if let Some(namespace_symbol) = self.resolve_namespace_member_symbol(
            ctx.tree_symbol_view(),
            expression_id,
            receiver_id,
            *member_key,
        ) && self.symbol_is_value_capable(ctx.profile, namespace_symbol)
        {
            return Ok(MemberResolution::Static {
                symbol: namespace_symbol,
            });
        }

        // resolve value-symbol static members for direct value receivers
        if let Some(member_symbol) =
            self.resolve_value_member_symbol(&mut ctx.reborrow(), receiver_id, member_key)?
        {
            return Ok(MemberResolution::Static {
                symbol: member_symbol,
            });
        }

        // prefer static-only lookup for direct class values
        let nominal_receiver = if let Some(nominal_symbol) = receiver_context.nominal_symbol {
            let mut visited = Vec::new();
            let member_symbol = self.resolve_member_symbol_for_symbol(
                ctx.compiler_context,
                ctx.module,
                ctx.module.id,
                ctx.profile,
                &ctx.index,
                ctx.tree,
                ctx.symbols,
                &*ctx.types,
                nominal_symbol,
                member_key,
                MemberLookupMode::Value,
                &mut visited,
            )?;
            if let Some(member_symbol) = member_symbol {
                return Ok(MemberResolution::Static {
                    symbol: member_symbol,
                });
            }
            true
        } else {
            false
        };

        // allow associated projection lookup for type-value receivers:
        // this covers both explicit static-argument projections and non-generic
        // owner projections like `Owner.AssociatedComptime`
        if (receiver_context.has_static_arguments || nominal_receiver)
            && let Some(selection) = self.select_associated_projection_member_symbol(
                &mut ctx.type_context_reborrow(),
                receiver_id,
                receiver_id,
                *member_key,
                Some(StaticMemberSymbolKind::AssociatedComptimeConst),
                true,
                true,
            )?
        {
            return Ok(MemberResolution::Static {
                symbol: selection.target_symbol,
            });
        }

        // nominal type values do not support instance member lookup in value mode
        if nominal_receiver {
            return Ok(MemberResolution::None);
        }

        // continue with regular member lookup
        self.resolve_member_symbol(ctx, receiver_ty, member_key)
    }

    /// Resolve one static member symbol from one direct value receiver expression.
    fn resolve_value_member_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let receiver_symbol = self
            .reference_symbol_for_expression(ctx.tree_symbol_view(), receiver_id)
            .or_else(|| ctx.tree.get(receiver_id).target_symbol());
        let Some(receiver_symbol) = receiver_symbol else {
            return Ok(None);
        };

        let receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        if !self.symbol_is_value_capable(ctx.profile, receiver_symbol) {
            return Ok(None);
        }

        let mut visited = Vec::new();
        self.resolve_member_symbol_for_symbol(
            ctx.compiler_context,
            ctx.module,
            ctx.module.id,
            ctx.profile,
            &ctx.index,
            ctx.tree,
            ctx.symbols,
            &*ctx.types,
            receiver_symbol,
            member_key,
            MemberLookupMode::Value,
            &mut visited,
        )
    }

    /// Return true when the expression is rooted at import.meta.
    pub(crate) fn is_import_meta_chain(
        &self,
        tree: &NodeTree,
        mut expression_id: LocalNodeId<Expression>,
    ) -> bool {
        loop {
            match tree.get(expression_id) {
                Expression::ImportMeta => return true,
                Expression::Member { left, .. } => {
                    expression_id = *left;
                }
                _ => return false,
            }
        }
    }

    /// Return true when the expression is rooted at import.meta.<member>.
    pub(crate) fn is_import_meta_chain_member(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        member: &str,
    ) -> bool {
        let member_key = self.repository.strings.intern(member);
        match tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                *name == Some(member_key) && self.is_import_meta_chain(tree, *left)
            }
            _ => false,
        }
    }

    /// Resolve member access through index signatures or missing-member diagnostics.
    pub(crate) fn resolve_member_index_or_missing(
        &self,
        ctx: &mut InferContext<'_>,
        context: MissingMemberResolutionContext<'_>,
    ) -> AnalyzeResult<LocalTypeId> {
        let MissingMemberResolutionContext {
            expression_id,
            receiver_id,
            receiver_ty_id,
            receiver_ty,
            receiver_requires_infer_convergence,
            allow_missing_member_deferral,
            force_unknown_receiver_diagnostic,
            member_key,
            member_resolution,
            is_surface_inference,
        } = context;

        let allow_associated_contract_blocker =
            self.is_projection_receiver_expression(&mut ctx.reborrow(), receiver_id);
        // infer index signature access for missing concrete members
        let mut index_visited = Vec::new();
        let index_signature_ty_id = self.resolve_index_signature_value_type_for_key(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            receiver_ty,
            member_key,
            &mut index_visited,
        );
        if let Some(index_signature_ty_id) = index_signature_ty_id {
            // enforce optional noPropertyAccessFromIndexSignature policy
            if ctx.options.no_property_access_from_index_signature
                && !self.is_import_meta_chain_member(ctx.tree, receiver_id, "env")
            {
                self.error(AnalyzeError::PropertyAccessFromIndexSignature {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                    receiver_ty: receiver_ty_id.into_global(ctx.module.id),
                    member_key: *member_key,
                });
            }

            // commit index-signature member resolution
            self.record_provisional_member_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                true,
                ctx.infer,
                ctx.types,
            );

            return Ok(index_signature_ty_id);
        }

        // keep surface inference diagnostics minimal until interface convergence
        if is_surface_inference {
            self.record_provisional_member_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                ctx.infer,
                ctx.types,
            );

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // defer missing-member diagnostics while receiver typing still depends on infer convergence
        let receiver_is_unannotated_parameter_reference =
            self.is_unannotated_parameter_receiver(ctx.type_view(), receiver_id);
        let receiver_is_indeterminate_for_callback_member_check = self
            .type_is_solver_placeholder(receiver_ty_id, ctx.types)
            || (matches!(
                ctx.types.get_type(receiver_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            ) && receiver_is_unannotated_parameter_reference);
        if allow_missing_member_deferral && receiver_requires_infer_convergence {
            ctx.infer
                .push_missing_member_obligation(MissingMemberObligation {
                    expression_id: expression_id.into_global_any(ctx.module.id),
                    receiver_expression_id: receiver_id.into_global_any(ctx.module.id),
                    receiver_type_id: receiver_ty_id,
                    member_key: *member_key,
                });

            self.record_provisional_member_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                ctx.infer,
                ctx.types,
            );

            let deferred_type_id = ctx.types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            );
            return Ok(deferred_type_id);
        }

        // keep callback-indeterminate receivers unresolved until callback inference converges
        if allow_missing_member_deferral && receiver_is_indeterminate_for_callback_member_check {
            self.record_provisional_member_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                ctx.infer,
                ctx.types,
            );

            let deferred_type_id = ctx.types.insert_type_from(Type::Error, expression_id);

            return Ok(deferred_type_id);
        }

        let diagnostic_receiver_ty_id = if force_unknown_receiver_diagnostic
            && !allow_missing_member_deferral
            && receiver_is_indeterminate_for_callback_member_check
        {
            ctx.types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            )
        } else {
            receiver_ty_id
        };

        // suppress missing-member cascades only when a primary semantic fault blocks lookup
        let reported = self.report_missing_member_diagnostic(
            ctx.type_view(),
            expression_id.into_any(),
            diagnostic_receiver_ty_id,
            *member_key,
            allow_associated_contract_blocker,
        )?;
        if !reported {
            self.record_provisional_member_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                ctx.infer,
                ctx.types,
            );

            return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
        }

        // commit unresolved member resolution for downstream consumers
        self.record_provisional_member_resolution(
            expression_id.into_global_any(ctx.module.id),
            Some(receiver_ty_id),
            member_resolution,
            None,
            None,
            false,
            ctx.infer,
            ctx.types,
        );

        Ok(ctx.types.insert_type_from(Type::Error, expression_id))
    }

    /// Return true when one receiver expression is an unannotated local parameter reference.
    fn is_unannotated_parameter_receiver(
        &self,
        ctx: TypeView<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(receiver_symbol) = ctx.tree.get(receiver_id).target_symbol() else {
            return false;
        };
        if receiver_symbol.module_id != ctx.module.id {
            return false;
        }

        let symbol_entry = ctx.symbols.get_symbol(receiver_symbol.local_id);
        if symbol_entry.binding_category != BindingCategory::Parameter {
            return false;
        }

        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return false;
        };
        if primary_declaration.module_id != ctx.module.id {
            return false;
        }
        if primary_declaration.local_id.ty != NodeType::Parameter {
            return false;
        }

        ctx.types
            .get_declared_type_id(primary_declaration)
            .is_none()
    }

    /// Resolve the member symbol for a type and member key.
    pub(crate) fn resolve_member_symbol_for_type(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        owner_module_id: ModuleId,
        profile: ProfileId,
        index: &AnalyzeIndex,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        receiver_ty: &Type,
        member_key: &StaticKey,
        visited: &mut Vec<GlobalSymbolId>,
        allow_implicit: bool,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // pick a lookup mode based on the receiver type
        let lookup_mode = MemberLookupMode::Instance;

        let resolved = match receiver_ty {
            Type::Value { .. } => {
                let Some(type_symbol) = self.get_language_symbol(profile, LanguageSymbol::Type)
                else {
                    return Ok(None);
                };
                self.resolve_member_symbol_for_symbol(
                    context,
                    module,
                    owner_module_id,
                    profile,
                    index,
                    tree,
                    symbols,
                    types,
                    type_symbol,
                    member_key,
                    lookup_mode,
                    visited,
                )?
            }
            Type::Intersection { elements } => {
                let element_ids = elements.clone();
                let mut resolved = None;
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    resolved = self.resolve_member_symbol_for_type(
                        context,
                        module,
                        owner_module_id,
                        profile,
                        index,
                        tree,
                        symbols,
                        types,
                        &element_ty,
                        member_key,
                        visited,
                        allow_implicit,
                    )?;
                    if resolved.is_some() {
                        break;
                    }
                }
                resolved
            }
            Type::Reference { symbol, .. } => {
                let resolved = self.resolve_member_symbol_for_symbol(
                    context,
                    module,
                    owner_module_id,
                    profile,
                    index,
                    tree,
                    symbols,
                    types,
                    *symbol,
                    member_key,
                    lookup_mode,
                    visited,
                )?;
                if resolved.is_some() {
                    resolved
                } else if self.symbol_is_static_parameter(
                    SymbolTypeView::new(context, module, profile, symbols, types),
                    *symbol,
                ) {
                    if let Some(constraint_type_id) =
                        types.get_static_parameter_constraint_type(*symbol)
                    {
                        let constraint_type = types.get_type(constraint_type_id).clone();
                        if let Type::Reference {
                            symbol: constraint_symbol,
                            ..
                        } = constraint_type
                            && constraint_symbol == *symbol
                        {
                            None
                        } else {
                            self.resolve_member_symbol_for_type(
                                context,
                                module,
                                owner_module_id,
                                profile,
                                index,
                                tree,
                                symbols,
                                types,
                                &constraint_type,
                                member_key,
                                visited,
                                allow_implicit,
                            )?
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        if resolved.is_some() || !allow_implicit {
            return Ok(resolved);
        }

        let Some(well_known_symbol) = self.well_known_symbol_for_type(receiver_ty, types) else {
            return Ok(None);
        };

        let mut symbol =
            self.resolve_implicit_well_known_carrier_symbol(profile, well_known_symbol);
        if symbol.is_none() && module.is_user() && self.options.load_libraries {
            self.require_library_environment(context.revision(), profile)
                .map_err(AnalyzeError::from)?;
            symbol = self.resolve_implicit_well_known_carrier_symbol(profile, well_known_symbol);
        }

        let Some(symbol) = symbol else {
            return Ok(None);
        };
        let symbol = self.remap_typevalue_symbol_to_type_space(
            ModuleSymbolView::new(context, module, profile, symbols),
            index,
            symbol,
        )?;
        self.resolve_member_symbol_for_symbol(
            context,
            module,
            owner_module_id,
            profile,
            index,
            tree,
            symbols,
            types,
            symbol,
            member_key,
            lookup_mode,
            visited,
        )
    }

    /// Resolve the member symbol for a nominal type symbol.
    pub(crate) fn resolve_member_symbol_for_symbol(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        owner_module_id: ModuleId,
        profile: ProfileId,
        index: &AnalyzeIndex,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // lookup precedence: declaration members, merge members, lineage members, then visible extensions
        // stop on cycles in symbol lookup
        if visited.contains(&symbol) {
            return Ok(None);
        }
        visited.push(symbol);

        // resolve members from the local module data
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            let allow_merge = module.language_type.supports_declaration_merging()
                || symbol_entry.origin.is_global_augmentation()
                || self.module_is_ambient_lib(module);
            let resolved = self.resolve_member_symbol_in_module(
                context,
                module,
                owner_module_id,
                profile,
                index,
                tree,
                symbols,
                types,
                symbol,
                member_key,
                lookup_mode,
                allow_merge,
                self.module_is_ambient_lib(module),
                visited,
            )?;
            if resolved.is_some() {
                return Ok(resolved);
            }

            // apply visible extensions only after declaration, merge, and lineage lookup
            return self.resolve_member_symbol_in_extensions(
                context,
                module,
                profile,
                index,
                symbols,
                types,
                owner_module_id,
                tree,
                symbol,
                member_key,
                lookup_mode,
            );
        }

        if self.module_is_ambient_lib(module) {
            return Ok(None);
        }

        let owner_dir = self
            .require_indexed_dir_declared(index, context.revision(), symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;
        let owner_symbol_entry = owner_dir.symbols.get_symbol(symbol.local_id);
        let remote_module = context.module(symbol.module_id);
        let remote_module = remote_module.as_ref();
        let is_ambient_lib = self.module_is_ambient_lib(remote_module);
        let allow_merge = remote_module.language_type.supports_declaration_merging()
            || owner_symbol_entry.origin.is_global_augmentation()
            || is_ambient_lib;
        let resolved = self.resolve_member_symbol_in_module(
            context,
            module,
            symbol.module_id,
            profile,
            index,
            &owner_dir.tree,
            &owner_dir.symbols,
            &owner_dir.types,
            symbol,
            member_key,
            lookup_mode,
            allow_merge,
            is_ambient_lib,
            visited,
        )?;
        if resolved.is_some() {
            return Ok(resolved);
        }

        // apply visible extensions only after remote declaration, merge, and lineage lookup
        self.resolve_member_symbol_in_extensions(
            context,
            module,
            profile,
            index,
            symbols,
            types,
            owner_module_id,
            tree,
            symbol,
            member_key,
            lookup_mode,
        )
    }

    /// Resolve member symbols using module-local declarations and merges.
    /// This follows infer_member_of_symbol lookup order using module data.
    pub(crate) fn resolve_member_symbol_in_module(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        owner_module_id: ModuleId,
        profile: ProfileId,
        index: &AnalyzeIndex,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        allow_merge: bool,
        owner_is_ambient_lib: bool,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // step 1: check members declared directly on this symbol
        if let Some(member_symbol) = self.find_member_symbol_in_declaration(
            context.revision(),
            owner_module_id,
            profile,
            tree,
            symbols,
            types,
            symbol,
            member_key,
            lookup_mode,
        ) {
            return Ok(Some(member_symbol));
        }

        // step 2: check merge groups and global augmentations
        if allow_merge {
            // scan merge group peers for members
            if let Some(group_id) = symbol_entry.merge_group {
                for group_symbol in symbols.merge_group_symbols(group_id) {
                    if *group_symbol == symbol.local_id {
                        continue;
                    }

                    if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                        context,
                        module,
                        owner_module_id,
                        profile,
                        index,
                        tree,
                        symbols,
                        types,
                        group_symbol.into_global(symbol.module_id),
                        member_key,
                        lookup_mode,
                        visited,
                    )? {
                        return Ok(Some(member_symbol));
                    }
                }
            }

            // scan global augmentations for additional members
            if let Some(key) = symbol_entry.key
                && (symbol_entry.origin.is_global_augmentation() || owner_is_ambient_lib)
            {
                let merge_category = match lookup_mode {
                    MemberLookupMode::Instance => GlobalMergeCategory::Instance,
                    MemberLookupMode::Value => GlobalMergeCategory::Value,
                    MemberLookupMode::Any => GlobalMergeCategory::Any,
                };
                let merge_symbols = self.collect_global_merge_sources_for_key(
                    context.revision(),
                    module,
                    index,
                    symbols,
                    profile,
                    key,
                    symbol_entry.space,
                    merge_category,
                )?;

                if !merge_symbols.is_empty() {
                    let mut seen = HashSet::new();
                    for merge_symbol in merge_symbols {
                        if !seen.insert(merge_symbol) || merge_symbol == symbol {
                            continue;
                        }

                        if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                            context,
                            module,
                            owner_module_id,
                            profile,
                            index,
                            tree,
                            symbols,
                            types,
                            merge_symbol,
                            member_key,
                            lookup_mode,
                            visited,
                        )? {
                            return Ok(Some(member_symbol));
                        }
                    }
                }
            }
        }

        // step 3: check inherited members
        let lineage = types.get_lineage_for_symbol(symbol).cloned();
        if let Some(lineage) = lineage {
            // follow extends first
            if let Some(extends) = lineage.extends
                && let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    context,
                    module,
                    owner_module_id,
                    profile,
                    index,
                    tree,
                    symbols,
                    types,
                    extends,
                    member_key,
                    lookup_mode,
                    visited,
                )?
            {
                return Ok(Some(member_symbol));
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    context,
                    module,
                    owner_module_id,
                    profile,
                    index,
                    tree,
                    symbols,
                    types,
                    *embedded,
                    member_key,
                    lookup_mode,
                    visited,
                )? {
                    return Ok(Some(member_symbol));
                }
            }
        }
        Ok(None)
    }

    /// Resolve members from extensions visible in the current module.
    pub(crate) fn resolve_member_symbol_in_extensions(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        index: &AnalyzeIndex,
        symbols: &SymbolTable,
        types: &TypeTable,
        owner_module_id: ModuleId,
        tree: &NodeTree,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // check visible extensions for this symbol
        let extension_symbols = self.visible_extension_symbols_for_target(
            SymbolTypeView::new(context, module, profile, symbols, types),
            symbol,
        )?;
        for extension_symbol in extension_symbols {
            let Some(extension) = self.extension_for_symbol_in_module(
                ModuleTypeView::new(context, module, profile, types),
                extension_symbol,
            )?
            else {
                continue;
            };
            if !self.is_extension_visible(context.revision(), module, profile, &extension)? {
                continue;
            }

            let member_symbol = self.find_member_symbol_in_extension(
                context,
                module,
                owner_module_id,
                profile,
                index,
                tree,
                symbols,
                types,
                extension_symbol,
                member_key,
                lookup_mode,
            )?;
            if let Some(member_symbol) = member_symbol {
                return Ok(Some(member_symbol));
            }
        }

        Ok(None)
    }

    /// Find a member symbol inside an extension declaration.
    pub(crate) fn find_member_symbol_in_extension(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        owner_module_id: ModuleId,
        profile: ProfileId,
        index: &AnalyzeIndex,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        extension_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // reuse local module data when the extension is local
        if extension_symbol.module_id == module.id {
            return Ok(self.find_member_symbol_in_declaration(
                context.revision(),
                owner_module_id,
                profile,
                tree,
                symbols,
                types,
                extension_symbol,
                member_key,
                lookup_mode,
            ));
        }

        if extension_symbol.module_id == owner_module_id {
            return Ok(self.find_member_symbol_in_declaration(
                context.revision(),
                extension_symbol.module_id,
                profile,
                tree,
                symbols,
                types,
                extension_symbol,
                member_key,
                lookup_mode,
            ));
        }

        self.require_remote_artifact_dir(
            context,
            owner_module_id,
            extension_symbol.module_id,
            profile,
            destack_artifact::ArtifactKey::dir_declared,
        )
        .map_err(AnalyzeError::from)?;
        let snapshot = self
            .require_indexed_dir_declared(
                index,
                context.revision(),
                extension_symbol.module_id,
                profile,
            )
            .map_err(AnalyzeError::from)?;
        Ok(self.find_member_symbol_in_declaration(
            context.revision(),
            extension_symbol.module_id,
            profile,
            &snapshot.tree,
            &snapshot.symbols,
            &snapshot.types,
            extension_symbol,
            member_key,
            lookup_mode,
        ))
    }

    /// Find a member symbol inside a declaration for a key.
    pub(crate) fn find_member_symbol_in_declaration(
        &self,
        revision: destack_workspace::Revision,
        owner_module_id: ModuleId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // collect primary and secondary declarations to scan
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        // scan declarations for a matching member
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let declaration = tree.get(declaration_id);

            if let Declaration::Enum(declaration) = declaration {
                // only expose enum fields through value lookups
                if matches!(lookup_mode, MemberLookupMode::Value | MemberLookupMode::Any) {
                    for field_id in &declaration.fields {
                        let field = tree.get(*field_id);
                        let field_key = match field.name {
                            Name::Identifier(name) | Name::String(name) => StaticKey::Name(name),
                            Name::Number(name) => StaticKey::Number(name),
                        };
                        if field_key.matches(member_key) {
                            let enum_scope = symbols.get_scope_by_symbol(symbol.local_id);
                            if let Some(field_symbol) =
                                symbols.find_active_symbol(enum_scope, field_key)
                            {
                                return Some(field_symbol.into_global(owner_module_id));
                            }
                        }
                    }
                }

                // check enum methods and members
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    // honor static versus instance lookup modes
                    if !self.member_visible_for_lookup(member, lookup_mode) {
                        continue;
                    }

                    let static_key = member.key().and_then(|key| {
                        self.static_key_from_key(revision, profile, tree, symbols, types, *key)
                    });

                    if let Some(static_key) = static_key
                        && static_key.matches(member_key)
                    {
                        return Some(member.symbol().into_global(owner_module_id));
                    }
                }

                continue;
            }

            // check type members on structured declarations
            let Some(members) = declaration.member_ids() else {
                continue;
            };

            for member_id in members {
                let member = tree.get(*member_id);
                // honor static versus instance lookup modes
                if !self.member_visible_for_lookup(member, lookup_mode) {
                    continue;
                }

                let static_key = member.key().and_then(|key| {
                    self.static_key_from_key(revision, profile, tree, symbols, types, *key)
                });

                if let Some(static_key) = static_key
                    && static_key.matches(member_key)
                {
                    return Some(member.symbol().into_global(owner_module_id));
                }
            }
        }

        None
    }

    /// Return true when a member matches the requested lookup mode.
    pub(crate) fn member_visible_for_lookup(
        &self,
        member: &Member,
        lookup_mode: MemberLookupMode,
    ) -> bool {
        // resolve the member staticness from the current dir shape
        let is_static = match member {
            Member::AssociatedType { is_static, .. }
            | Member::AssociatedConst { is_static, .. }
            | Member::Field { is_static, .. }
            | Member::Method { is_static, .. }
            | Member::Embed { is_static, .. } => *is_static,
            Member::StaticBlock { .. } => true,
            Member::ComptimeBlock { .. } | Member::Error { .. } => false,
        };

        // match member staticness to the requested lookup mode
        match lookup_mode {
            MemberLookupMode::Any => true,
            MemberLookupMode::Instance => !is_static,
            MemberLookupMode::Value => is_static,
        }
    }
}
