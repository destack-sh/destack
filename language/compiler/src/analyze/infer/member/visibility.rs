use super::*;

impl Compiler {
    /// Check visibility constraints for a resolved member access.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn check_member_resolution_visibility(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_resolution: &MemberResolution,
        member_symbol: Option<GlobalSymbolId>,
        receiver_ty_id: LocalTypeId,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        match member_resolution {
            MemberResolution::Dynamic { candidates } => {
                for candidate in candidates {
                    self.check_member_visibility(
                        module,
                        expression_id,
                        candidate.symbol,
                        candidate.receiver_ty_id,
                        profile,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            _ => {
                if let Some(member_symbol) = member_symbol {
                    self.check_member_visibility(
                        module,
                        expression_id,
                        member_symbol,
                        receiver_ty_id,
                        profile,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                    return Ok(());
                }

                let Some(receiver_symbol) =
                    self.receiver_symbol_for_visibility(receiver_ty_id, types)
                else {
                    return Ok(());
                };
                let Some(context) = self.parameter_property_member_context_for_key(
                    module,
                    profile,
                    receiver_symbol,
                    member_key,
                    tree,
                    symbols,
                    types,
                )?
                else {
                    return Ok(());
                };
                self.check_visibility_context(
                    module,
                    expression_id,
                    receiver_symbol,
                    receiver_ty_id,
                    profile,
                    symbols,
                    types,
                    ctx,
                    MemberVisibilityContext {
                        visibility: context.visibility,
                        owner_symbol: context.owner_symbol,
                    },
                );
            }
        }

        Ok(())
    }

    /// Resolve the visibility context for a member symbol.
    pub(crate) fn member_visibility_context_for_symbol(
        &self,
        module: &Module,
        member_symbol: GlobalSymbolId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<MemberVisibilityContext>> {
        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            member_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                self.member_visibility_context_for_symbol_in_tree(
                    owner_module.id,
                    member_symbol,
                    owner_tree,
                    owner_symbols,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the visibility context for a member symbol inside a known tree.
    pub(crate) fn member_visibility_context_for_symbol_in_tree(
        &self,
        module_id: ModuleId,
        member_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<MemberVisibilityContext> {
        // resolve the member symbol entry
        let member_entry = symbols.get_symbol(member_symbol.local_id);
        let member_node = member_entry.primary_declaration?;
        let member_local = member_node.local_id;
        if member_local.ty != NodeType::Member {
            return None;
        }

        // resolve the owning declaration symbol
        let scope = symbols.get_scope_by_id(member_entry.scope.0);
        let owner_id = scope.owner_id?;
        let owner_entry = symbols.get_symbol(owner_id);
        if !matches!(owner_entry.ty, SymbolType::Class | SymbolType::Struct) {
            return None;
        }

        let owner_symbol = owner_id.with_type(owner_entry.ty).into_global(module_id);
        let member_id = member_local.into_typed::<Member>();
        let member = tree.get(member_id);

        // extract visibility from modifiers
        let modifiers = match member {
            Member::Type { modifiers, .. }
            | Member::ComptimeConst { modifiers, .. }
            | Member::Field { modifiers, .. }
            | Member::Method { modifiers, .. }
            | Member::Embed { modifiers, .. }
            | Member::StaticBlock { modifiers, .. }
            | Member::ComptimeBlock { modifiers, .. } => modifiers.as_ref(),
        };
        let visibility = modifiers
            .and_then(|modifier| modifier.visibility)
            .unwrap_or(Visibility::Public);

        Some(MemberVisibilityContext {
            visibility,
            owner_symbol,
        })
    }

    /// Enforce visibility for a resolved member symbol.
    pub(crate) fn check_member_visibility(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        receiver_ty_id: LocalTypeId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<()> {
        let Some(context) = self.member_visibility_context_for_symbol(
            module,
            member_symbol,
            profile,
            tree,
            symbols,
        )?
        else {
            return Ok(());
        };

        self.check_visibility_context(
            module,
            expression_id,
            member_symbol,
            receiver_ty_id,
            profile,
            symbols,
            types,
            ctx,
            context,
        );

        Ok(())
    }

    /// Enforce member visibility for a resolved visibility context.
    pub(crate) fn check_visibility_context(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        receiver_ty_id: LocalTypeId,
        profile: ProfileId,
        symbols: &SymbolTable,
        types: &TypeTable,
        ctx: &InferContext,
        context: MemberVisibilityContext,
    ) {
        // public members are always accessible
        if context.visibility == Visibility::Public {
            return;
        }

        // require a class/struct context for private and protected access
        let Some(current_class) = ctx.in_nominal_symbol else {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        };

        // private members require the declaring class
        if context.visibility == Visibility::Private && current_class != context.owner_symbol {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        }

        // protected members require a subclass context
        if context.visibility == Visibility::Protected
            && current_class != context.owner_symbol
            && !self.is_type_lineage_assignable(
                module,
                profile,
                current_class,
                context.owner_symbol,
                symbols,
                types,
            )
        {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        }

        // protected members must be accessed through the current class lineage
        if context.visibility == Visibility::Protected {
            let receiver_symbol = self.receiver_symbol_for_visibility(receiver_ty_id, types);
            if let Some(receiver_symbol) = receiver_symbol
                && receiver_symbol != current_class
                && !self.is_type_lineage_assignable(
                    module,
                    profile,
                    receiver_symbol,
                    current_class,
                    symbols,
                    types,
                )
            {
                self.error(AnalyzeError::InaccessibleSymbol {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    visibility: context.visibility,
                    symbol: member_symbol,
                });
            }
        }
    }

    /// Resolve visibility metadata for constructor parameter properties by key.
    pub(crate) fn parameter_property_member_context_for_key(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<ParameterPropertyMemberContext>> {
        let StaticKey::Name(member_name) = member_key else {
            return Ok(None);
        };

        // walk the receiver lineage and find the first parameter property with this key
        let mut current_symbol = Some(receiver_symbol);
        while let Some(owner_symbol) = current_symbol {
            let context = self
                .with_module_tree_symbols_or_local_at_stage(
                    module,
                    profile,
                    owner_symbol.module_id,
                    tree,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |_, owner_tree, owner_symbols| {
                        let owner_entry = owner_symbols.get_symbol(owner_symbol.local_id);
                        let declaration_id = owner_entry.primary_declaration?.local_id;
                        if declaration_id.ty != NodeType::Declaration {
                            return None;
                        }

                        let declaration =
                            owner_tree.get(declaration_id.into_typed::<Declaration>());
                        let members = declaration.member_ids()?;
                        for member_id in members {
                            let Member::Method {
                                signature,
                                modifiers,
                                ..
                            } = owner_tree.get(*member_id)
                            else {
                                continue;
                            };
                            if signature.mode != Some(destack_dir::FunctionMode::Constructor) {
                                continue;
                            }
                            if modifiers.as_ref().is_some_and(|modifier| {
                                modifier.anchor == Some(BindingAnchor::Static)
                            }) {
                                continue;
                            }

                            for parameter_id in &signature.dynamic_parameters {
                                let parameter = owner_tree.get(*parameter_id);
                                let Parameter::Named {
                                    name, modifiers, ..
                                } = parameter
                                else {
                                    continue;
                                };
                                if name != member_name {
                                    continue;
                                }

                                let Some(modifiers) = modifiers.as_ref() else {
                                    continue;
                                };
                                let is_parameter_property = modifiers.visibility.is_some()
                                    || modifiers.mutability == Some(Mutability::Immutable);
                                if !is_parameter_property {
                                    continue;
                                }

                                return Some(ParameterPropertyMemberContext {
                                    visibility: modifiers.visibility.unwrap_or(Visibility::Public),
                                    is_readonly: modifiers.mutability
                                        == Some(Mutability::Immutable),
                                    owner_symbol,
                                });
                            }
                        }

                        None
                    },
                )
                .map_err(AnalyzeError::from)?;

            if let Some(context) = context {
                return Ok(Some(context));
            }

            current_symbol = self
                .with_module_types_or_local_at_stage(
                    module,
                    profile,
                    owner_symbol.module_id,
                    types,
                    AnalyzeDependencyStage::Declare,
                    |_, owner_types| {
                        owner_types
                            .get_lineage_for_symbol(owner_symbol)
                            .and_then(|lineage| lineage.extends)
                    },
                )
                .map_err(AnalyzeError::from)?;
        }

        Ok(None)
    }

    /// Resolve a nominal symbol for visibility checks from a receiver type.
    pub(crate) fn receiver_symbol_for_visibility(
        &self,
        receiver_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        // resolve nominal symbols for reference-like receiver types
        match types.get_type(receiver_ty_id) {
            Type::Reference { symbol, .. } => Some(*symbol),
            Type::Value { value } => types.get_type(*value).symbol(),
            _ => None,
        }
    }
}
