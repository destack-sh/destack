use super::*;
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{AnalyzeDependencyStage, CanonicalSymbolMode};
use crate::analyze::module::GlobalMergeCategory;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the preferred member type for a symbol-aware lookup.
    pub(crate) fn resolve_member_type_for_symbol(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        inferred_member_ty_id: Option<LocalTypeId>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(member_symbol) = member_symbol else {
            return Ok(inferred_member_ty_id);
        };
        let member_symbol = self.normalize_member_symbol_for_declare_reads(
            module,
            profile,
            member_symbol,
            symbols,
        )?;

        let mut member_ty_id = match (
            types.get_value_type_id(member_symbol),
            inferred_member_ty_id,
        ) {
            (Some(value_ty_id), Some(inferred_member_ty_id)) => {
                if self.is_infer_var_type(value_ty_id, types) {
                    Some(inferred_member_ty_id)
                } else {
                    Some(value_ty_id)
                }
            }
            (Some(value_ty_id), None) => Some(value_ty_id),
            (None, Some(inferred_member_ty_id)) => Some(inferred_member_ty_id),
            (None, None) => None,
        };

        // import remote member types when local tables have no value type yet
        if member_ty_id.is_none() && member_symbol.module_id != module.id {
            let member_kind = self.query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                member_symbol,
                tree,
                symbols,
            )?;
            if member_kind == Some(StaticMemberSymbolKind::AssociatedComptimeConst) {
                let associated_type_id = self.query_associated_comptime_member_type_for_symbol(
                    module,
                    profile,
                    expression_id,
                    member_symbol,
                    tree,
                    symbols,
                    types,
                )?;
                member_ty_id = associated_type_id.or(member_ty_id);
                return Ok(member_ty_id);
            }

            let remote_ty_id = self.resolve_remote_symbol_value_type_for_interface(
                module,
                profile,
                expression_id.into_any(),
                member_symbol,
                types,
            )?;
            member_ty_id = Some(remote_ty_id);
        }

        Ok(member_ty_id)
    }

    /// Normalize one member symbol to a declaration-backed symbol for declare reads.
    fn normalize_member_symbol_for_declare_reads(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<GlobalSymbolId> {
        let mut current_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            member_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let mut visited_symbols = HashSet::new();

        loop {
            if !visited_symbols.insert(current_symbol) {
                return Ok(current_symbol);
            }

            let (normalized_symbol, has_concrete_primary_declaration, next_symbol) = self
                .with_module_symbols_or_local_at_stage(
                    module,
                    profile,
                    current_symbol.module_id,
                    symbols,
                    AnalyzeDependencyStage::Declare,
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
            current_symbol = self.canonical_symbol_id(
                module,
                symbols,
                profile,
                next_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
        }
    }

    /// Resolve one associated comptime member type without generic remote value import.
    #[allow(clippy::too_many_arguments)]
    fn query_associated_comptime_member_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        if member_symbol.module_id == module.id {
            if let Some(value_type_id) = types.get_value_type_id(member_symbol) {
                return Ok(Some(value_type_id));
            }

            let symbol_entry = symbols.get_symbol(member_symbol.local_id);
            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                return Ok(None);
            };
            if let Some(signature_type_id) = types.get_signature_type_for_node(primary_declaration)
            {
                return Ok(Some(signature_type_id));
            }
            if let Some(declared_type_id) = types.get_declared_type_id(primary_declaration) {
                return Ok(Some(declared_type_id));
            }

            if primary_declaration.local_id.ty == NodeType::Member {
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                if let Member::ComptimeConst {
                    ty: Some(member_type),
                    ..
                } = tree.get(member_id)
                {
                    if let Some(declared_type_id) = types.get_declared_type_id(
                        member_type.into_global_any(primary_declaration.module_id),
                    ) {
                        return Ok(Some(declared_type_id));
                    }

                    let declared_type_id = self.resolve_declared_type_expression(
                        module,
                        profile,
                        *member_type,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )?;
                    return Ok(Some(declared_type_id));
                }
            }
            return Ok(None);
        }

        let remote_import = self
            .with_module_tree_symbols_at_stage(
                module,
                profile,
                member_symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |remote_module,
                 remote_tree,
                 remote_symbols|
                 -> AnalyzeResult<Option<(GlobalSymbolId, Type, TypeTable)>> {
                    let remote_types = remote_module.dir(profile).types.read();
                    let mut remote_snapshot = remote_types.clone();
                    let remote_symbol_entry = remote_symbols.get_symbol(member_symbol.local_id);
                    let resolved_symbol = GlobalSymbolId::new(
                        remote_module.id,
                        member_symbol.local_id.with_type(remote_symbol_entry.ty),
                    );

                    let mut remote_type_id = remote_snapshot.get_value_type_id(resolved_symbol);
                    if remote_type_id.is_none()
                        && let Some(primary_declaration) = remote_symbol_entry.primary_declaration
                    {
                        remote_type_id =
                            remote_snapshot.get_signature_type_for_node(primary_declaration);
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
                        if let Member::ComptimeConst {
                            ty: Some(member_type),
                            ..
                        } = remote_tree.get(member_id)
                        {
                            remote_type_id = remote_snapshot.get_declared_type_id(
                                member_type.into_global_any(primary_declaration.module_id),
                            );
                            if remote_type_id.is_none() {
                                let evaluated_type_id = self.resolve_declared_type_expression(
                                    remote_module,
                                    profile,
                                    *member_type,
                                    remote_tree,
                                    remote_symbols,
                                    &mut remote_snapshot,
                                    true,
                                    true,
                                )?;
                                remote_type_id = Some(evaluated_type_id);
                            }
                        }
                    }

                    let Some(remote_type_id) = remote_type_id else {
                        return Ok(None);
                    };

                    let remote_type = remote_snapshot.get_type(remote_type_id).clone();
                    Ok(Some((resolved_symbol, remote_type, remote_snapshot)))
                },
            )
            .map_err(AnalyzeError::from)?;
        let remote_import = remote_import?;

        let Some((resolved_symbol, remote_type, remote_snapshot)) = remote_import else {
            return Ok(None);
        };

        let local_type_id = self.import_type_from_remote_for_node(
            expression_id.into_any(),
            &remote_type,
            &remote_snapshot,
            resolved_symbol,
            types,
        );
        Ok(Some(local_type_id))
    }

    /// Resolve member symbols for a receiver type when nominal dispatch is possible.
    pub(crate) fn resolve_member_symbol(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<MemberResolution> {
        let resolution = match receiver_ty {
            Type::Reference { .. } => {
                // resolve nominal members first
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    module,
                    receiver_ty,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
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
                    module,
                    receiver_ty,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
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
                    let element_ty = types.get_type(*element_id).clone();
                    let mut visited = Vec::new();
                    let mut member_symbol = self.resolve_member_symbol_for_type(
                        module,
                        &element_ty,
                        member_key,
                        profile,
                        tree,
                        symbols,
                        types,
                        &mut visited,
                        true,
                    )?;
                    if member_symbol.is_none() {
                        // fall back to instance type owners when possible
                        if let Some(instance_symbol) = types.symbol_for_instance_type(*element_id) {
                            member_symbol = self.resolve_member_symbol_for_symbol(
                                module,
                                instance_symbol,
                                member_key,
                                MemberLookupMode::Instance,
                                profile,
                                tree,
                                symbols,
                                types,
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
                    module,
                    receiver_ty,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
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
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        receiver_context: &MemberReceiverContext,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<MemberResolution> {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);

        // resolve namespace import member paths before receiver-type dispatch
        if let Some(namespace_symbol) = self.resolve_namespace_member_symbol(
            module,
            profile,
            expression_id,
            receiver_id,
            *member_key,
            tree,
            symbols,
        ) && self.symbol_is_value_capable(profile, namespace_symbol)
        {
            return Ok(MemberResolution::Static {
                symbol: namespace_symbol,
            });
        }

        // resolve value-symbol static members for direct value receivers
        if let Some(member_symbol) = self.resolve_value_member_symbol_for_receiver_expression(
            module,
            receiver_id,
            member_key,
            profile,
            tree,
            symbols,
            types,
        )? {
            return Ok(MemberResolution::Static {
                symbol: member_symbol,
            });
        }

        // prefer static-only lookup for direct class values
        let nominal_receiver = if let Some(nominal_symbol) = receiver_context.nominal_symbol {
            let mut visited = Vec::new();
            let member_symbol = self.resolve_member_symbol_for_symbol(
                module,
                nominal_symbol,
                member_key,
                MemberLookupMode::Value,
                profile,
                tree,
                symbols,
                types,
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
                module,
                profile,
                receiver_id,
                receiver_id,
                *member_key,
                Some(StaticMemberSymbolKind::AssociatedComptimeConst),
                tree,
                symbols,
                types,
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
        self.resolve_member_symbol(
            module,
            receiver_ty,
            member_key,
            profile,
            tree,
            symbols,
            types,
        )
    }

    /// Resolve one static member symbol from one direct value receiver expression.
    #[allow(clippy::too_many_arguments)]
    fn resolve_value_member_symbol_for_receiver_expression(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let receiver_symbol = self
            .reference_symbol_for_expression(module, receiver_id, profile, tree, symbols)
            .or_else(|| tree.get(receiver_id).target_symbol());
        let Some(receiver_symbol) = receiver_symbol else {
            return Ok(None);
        };

        let receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        if !self.symbol_is_value_capable(profile, receiver_symbol) {
            return Ok(None);
        }

        let mut visited = Vec::new();
        self.resolve_member_symbol_for_symbol(
            module,
            receiver_symbol,
            member_key,
            MemberLookupMode::Value,
            profile,
            tree,
            symbols,
            types,
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
        let member_key = self.program.strings.intern(member);
        match tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                *name == member_key && self.is_import_meta_chain(tree, *left)
            }
            _ => false,
        }
    }

    /// Resolve member access through index signatures or missing-member diagnostics.
    pub(crate) fn resolve_member_index_or_missing(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        receiver_requires_infer_convergence: bool,
        allow_missing_member_deferral: bool,
        member_key: &StaticKey,
        member_resolution: &MemberResolution,
        profile: ProfileId,
        is_surface_inference: bool,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer index signature access for missing concrete members
        let mut index_visited = Vec::new();
        let index_signature_ty_id = self.resolve_index_signature_value_type_for_key(
            module,
            profile,
            expression_id.into_any(),
            symbols,
            receiver_ty,
            member_key,
            types,
            &mut index_visited,
        );
        if let Some(index_signature_ty_id) = index_signature_ty_id {
            // enforce optional noPropertyAccessFromIndexSignature policy
            if options.no_property_access_from_index_signature
                && !self.is_import_meta_chain_member(tree, receiver_id, "env")
            {
                self.error(AnalyzeError::PropertyAccessFromIndexSignature {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    receiver_ty: receiver_ty_id.into_global(module.id),
                    member_key: member_key.clone(),
                });
            }

            // commit index-signature member resolution
            self.record_provisional_member_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                true,
                infer,
                types,
            );

            return Ok(index_signature_ty_id);
        }

        // keep surface inference diagnostics minimal until interface convergence
        if is_surface_inference {
            self.record_provisional_member_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                infer,
                types,
            );

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // defer missing-member diagnostics while receiver typing still depends on infer convergence
        let receiver_is_unannotated_parameter_reference = self
            .receiver_expression_is_unannotated_parameter_reference(
                module,
                receiver_id,
                tree,
                symbols,
                types,
            );
        let receiver_is_indeterminate_for_callback_member_check = self
            .type_is_solver_placeholder(receiver_ty_id, types)
            || (matches!(
                types.get_type(receiver_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            ) && receiver_is_unannotated_parameter_reference);
        if allow_missing_member_deferral && receiver_requires_infer_convergence {
            infer.push_missing_member_obligation(MissingMemberObligation {
                expression_id: expression_id.into_global_any(module.id),
                receiver_expression_id: receiver_id.into_global_any(module.id),
                receiver_type_id: receiver_ty_id,
                member_key: member_key.clone(),
            });

            self.record_provisional_member_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                infer,
                types,
            );

            let deferred_type_id = types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            );
            return Ok(deferred_type_id);
        }

        // keep callback-indeterminate receivers unresolved until callback replay lands
        if allow_missing_member_deferral && receiver_is_indeterminate_for_callback_member_check {
            self.record_provisional_member_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                infer,
                types,
            );

            let deferred_type_id = types.insert_type_from(Type::Error, expression_id);

            return Ok(deferred_type_id);
        }

        // suppress missing-member cascades only when a primary semantic fault blocks lookup
        let allow_associated_contract_blocker = self
            .query_expression_is_projection_receiver_for_infer(
                module,
                profile,
                receiver_id,
                tree,
                symbols,
                types,
            );
        let reported = self.report_missing_member_diagnostic_for_receiver_type(
            module,
            profile,
            expression_id,
            receiver_ty_id,
            member_key.clone(),
            symbols,
            types,
            allow_associated_contract_blocker,
        )?;
        if !reported {
            self.record_provisional_member_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                member_resolution,
                None,
                None,
                false,
                infer,
                types,
            );

            return Ok(types.insert_type_from(Type::Error, expression_id));
        }

        // keep unresolved member resolution for downstream consumers
        // and preserve unknown typing after a reported missing member

        // commit unresolved member resolution for downstream consumers
        self.record_provisional_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            member_resolution,
            None,
            None,
            false,
            infer,
            types,
        );

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Return true when one receiver expression is an unannotated local parameter reference.
    fn receiver_expression_is_unannotated_parameter_reference(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        let Some(receiver_symbol) = tree.get(receiver_id).target_symbol() else {
            return false;
        };
        if receiver_symbol.module_id != module.id {
            return false;
        }

        let symbol_entry = symbols.get_symbol(receiver_symbol.local_id);
        if symbol_entry.binding_category != BindingCategory::Parameter {
            return false;
        }

        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return false;
        };
        if primary_declaration.module_id != module.id {
            return false;
        }
        if primary_declaration.local_id.ty != NodeType::Parameter {
            return false;
        }

        types.get_declared_type_id(primary_declaration).is_none()
    }

    /// Resolve the member symbol for a type and member key.
    pub(crate) fn resolve_member_symbol_for_type(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
        allow_implicit: bool,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // pick a lookup mode based on the receiver type
        let lookup_mode = self.query_member_lookup_mode_for_type(receiver_ty);

        let resolved = match receiver_ty {
            Type::Value { .. } => {
                let Some(type_symbol) = self.get_language_symbol(profile, LanguageSymbol::Type)
                else {
                    return Ok(None);
                };
                self.resolve_member_symbol_for_symbol(
                    module,
                    type_symbol,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            }
            Type::Intersection { elements } => {
                let element_ids = elements.clone();
                let mut resolved = None;
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    resolved = self.resolve_member_symbol_for_type(
                        module,
                        &element_ty,
                        member_key,
                        profile,
                        tree,
                        symbols,
                        types,
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
                    module,
                    *symbol,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                )?;
                if resolved.is_some() {
                    resolved
                } else if self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
                {
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
                                module,
                                &constraint_type,
                                member_key,
                                profile,
                                tree,
                                symbols,
                                types,
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

        let Some(symbol) = self.get_well_known_type_symbol(profile, well_known_symbol) else {
            return Ok(None);
        };
        let symbol = self
            .remap_typevalue_symbol_to_canonical_type_space(module, profile, symbol)
            .map_err(AnalyzeError::from)?;
        self.resolve_member_symbol_for_symbol(
            module,
            symbol,
            member_key,
            lookup_mode,
            profile,
            tree,
            symbols,
            types,
            visited,
        )
    }

    /// Resolve the member symbol for a nominal type symbol.
    pub(crate) fn resolve_member_symbol_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
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
                module,
                symbol,
                member_key,
                lookup_mode,
                profile,
                tree,
                symbols,
                types,
                allow_merge,
                self.module_is_ambient_lib(module),
                visited,
            )?;
            if resolved.is_some() {
                return Ok(resolved);
            }

            // apply visible extensions only after declaration, merge, and lineage lookup
            return self.resolve_member_symbol_in_extensions(
                module,
                symbol,
                member_key,
                lookup_mode,
                profile,
                tree,
                symbols,
                types,
            );
        }

        if self.module_is_ambient_lib(module) {
            return Ok(None);
        }

        let resolved = self
            .with_module_tree_symbols_at_stage(
                module,
                profile,
                symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let owner_types = owner_module.dir(profile).types.read();
                    let owner_symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                    let allow_merge = owner_module.language_type.supports_declaration_merging()
                        || owner_symbol_entry.origin.is_global_augmentation()
                        || self.module_is_ambient_lib(owner_module);
                    self.resolve_member_symbol_in_module(
                        module,
                        symbol,
                        member_key,
                        lookup_mode,
                        profile,
                        owner_tree,
                        owner_symbols,
                        &owner_types,
                        allow_merge,
                        self.module_is_ambient_lib(owner_module),
                        visited,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;
        let resolved = resolved?;
        if resolved.is_some() {
            return Ok(resolved);
        }

        // apply visible extensions only after remote declaration, merge, and lineage lookup
        self.resolve_member_symbol_in_extensions(
            module,
            symbol,
            member_key,
            lookup_mode,
            profile,
            tree,
            symbols,
            types,
        )
    }

    /// Resolve member symbols using module-local declarations and merges.
    /// This follows infer_member_of_symbol lookup order using module data.
    pub(crate) fn resolve_member_symbol_in_module(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        allow_merge: bool,
        owner_is_ambient_lib: bool,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // step 1: check members declared directly on this symbol
        if let Some(member_symbol) = self.find_member_symbol_in_declaration(
            symbol.module_id,
            profile,
            symbol,
            member_key,
            lookup_mode,
            tree,
            symbols,
            types,
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
                        module,
                        group_symbol.into_global(symbol.module_id),
                        member_key,
                        lookup_mode,
                        profile,
                        tree,
                        symbols,
                        types,
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
                    module,
                    profile,
                    key,
                    symbol_entry.space,
                    merge_category,
                );

                if !merge_symbols.is_empty() {
                    let mut seen = HashSet::new();
                    for merge_symbol in merge_symbols {
                        if !seen.insert(merge_symbol) || merge_symbol == symbol {
                            continue;
                        }

                        if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                            module,
                            merge_symbol,
                            member_key,
                            lookup_mode,
                            profile,
                            tree,
                            symbols,
                            types,
                            visited,
                        )? {
                            return Ok(Some(member_symbol));
                        }
                    }
                }
            }
        }

        // step 3: check inherited members
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            // follow extends first
            if let Some(extends) = lineage.extends
                && let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    extends,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            {
                return Ok(Some(member_symbol));
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    *embedded,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
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
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // check visible extensions for this symbol
        let extension_symbols =
            self.visible_extension_symbols_for_target(module, profile, symbols, types, symbol)?;
        for extension_symbol in extension_symbols {
            let Some(extension) =
                self.extension_for_symbol_in_module(module, profile, extension_symbol, types)?
            else {
                continue;
            };
            if !self.is_extension_visible(module, &extension) {
                continue;
            }

            let member_symbol = self.find_member_symbol_in_extension(
                module,
                profile,
                extension_symbol,
                member_key,
                lookup_mode,
                tree,
                symbols,
                types,
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
        module: &Module,
        profile: ProfileId,
        extension_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // reuse local module data when the extension is local
        if extension_symbol.module_id == module.id {
            return Ok(self.find_member_symbol_in_declaration(
                module.id,
                profile,
                extension_symbol,
                member_key,
                lookup_mode,
                tree,
                symbols,
                types,
            ));
        }

        self.with_module_tree_symbols_at_stage(
            module,
            profile,
            extension_symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                let owner_types = owner_module.dir(profile).types.read();
                self.find_member_symbol_in_declaration(
                    owner_module.id,
                    profile,
                    extension_symbol,
                    member_key,
                    lookup_mode,
                    owner_tree,
                    owner_symbols,
                    &owner_types,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Find a member symbol inside a declaration for a key.
    pub(crate) fn find_member_symbol_in_declaration(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
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

            if let Declaration::Enum {
                fields, members, ..
            } = declaration
            {
                // only expose enum fields through value lookups
                if matches!(lookup_mode, MemberLookupMode::Value | MemberLookupMode::Any) {
                    for field_id in fields {
                        let field = tree.get(*field_id);
                        let field_key = StaticKey::Name(field.name);
                        if field_key.matches(member_key) {
                            return Some(field.symbol.into_global(module_id));
                        }
                    }
                }

                // check enum methods and members
                for member_id in members {
                    let member = tree.get(*member_id);
                    // honor static versus instance lookup modes
                    if !self.member_visible_for_lookup(member, lookup_mode) {
                        continue;
                    }

                    let static_key = member.key().and_then(|key| {
                        self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
                    });

                    if let Some(static_key) = static_key
                        && static_key.matches(member_key)
                    {
                        return Some(member.symbol().into_global(module_id));
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
                    self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
                });

                if let Some(static_key) = static_key
                    && static_key.matches(member_key)
                {
                    return Some(member.symbol().into_global(module_id));
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
        // resolve modifiers from the member node
        let modifiers = match member {
            Member::Type { modifiers, .. }
            | Member::ComptimeConst { modifiers, .. }
            | Member::Field { modifiers, .. }
            | Member::Method { modifiers, .. }
            | Member::Embed { modifiers, .. }
            | Member::StaticBlock { modifiers, .. }
            | Member::ComptimeBlock { modifiers, .. } => modifiers.as_ref(),
        };

        // filter by anchor for the lookup mode
        self.modifiers_visible_for_lookup(modifiers, lookup_mode)
    }

    /// Return true when modifiers allow access for a lookup mode.
    pub(crate) fn modifiers_visible_for_lookup(
        &self,
        modifiers: Option<&BindingModifier>,
        lookup_mode: MemberLookupMode,
    ) -> bool {
        // normalize the binding anchor for comparison
        let anchor = modifiers.and_then(|modifiers| modifiers.anchor);

        // match anchors to the requested lookup mode
        match lookup_mode {
            MemberLookupMode::Any => true,
            MemberLookupMode::Instance => !matches!(anchor, Some(BindingAnchor::Static)),
            MemberLookupMode::Value => matches!(anchor, Some(BindingAnchor::Static)),
        }
    }
}
