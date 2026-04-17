use super::*;
use crate::analyze::common::{InferContext, TreeSymbolView};
use destack_dir::FunctionMode;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check visibility constraints for a resolved member access.
    pub(crate) fn check_member_resolution_visibility(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_resolution: &MemberResolution,
        member_symbol: Option<GlobalSymbolId>,
        receiver_ty_id: LocalTypeId,
        member_key: &StaticKey,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        match member_resolution {
            MemberResolution::Dynamic { candidates } => {
                for candidate in candidates {
                    self.check_member_visibility(
                        &mut ctx.reborrow(),
                        expression_id,
                        candidate.symbol,
                        candidate.receiver_ty_id,
                        state,
                    )?;
                }
            }
            _ => {
                if let Some(member_symbol) = member_symbol {
                    self.check_member_visibility(
                        &mut ctx.reborrow(),
                        expression_id,
                        member_symbol,
                        receiver_ty_id,
                        state,
                    )?;
                    return Ok(());
                }

                let Some(receiver_symbol) =
                    self.receiver_symbol_for_visibility(receiver_ty_id, ctx.types)
                else {
                    return Ok(());
                };
                let Some(context) = self.parameter_property_member_context_for_key(
                    &mut ctx.reborrow(),
                    receiver_symbol,
                    member_key,
                )?
                else {
                    return Ok(());
                };
                self.check_visibility_context(
                    ctx,
                    expression_id,
                    receiver_symbol,
                    receiver_ty_id,
                    state,
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
        ctx: TreeSymbolView<'_>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<MemberVisibilityContext>> {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            member_symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| self.member_visibility_context_for_symbol_in_tree(view, member_symbol),
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the visibility context for a member symbol inside a known tree.
    pub(crate) fn member_visibility_context_for_symbol_in_tree(
        &self,
        ctx: TreeSymbolView<'_>,
        member_symbol: GlobalSymbolId,
    ) -> Option<MemberVisibilityContext> {
        // resolve the member symbol entry
        let member_entry = ctx.symbols.get_symbol(member_symbol.local_id);
        let member_node = member_entry.primary_declaration?;
        let member_local = member_node.local_id;
        if member_local.ty != NodeType::Member {
            return None;
        }

        // resolve the owning declaration symbol
        let scope = ctx.symbols.get_scope_by_id(member_entry.scope.0);
        let owner_id = scope.owner_id?;
        let owner_entry = ctx.symbols.get_symbol(owner_id);
        if !matches!(owner_entry.ty, SymbolType::Class | SymbolType::Struct) {
            return None;
        }

        let owner_symbol = owner_id
            .with_type(owner_entry.ty)
            .into_global(ctx.module.id);
        let member_id = member_local.into_typed::<Member>();
        let member = ctx.tree.get(member_id);

        // extract visibility from the current member shape
        let visibility = match member {
            Member::AssociatedType { visibility, .. }
            | Member::AssociatedConst { visibility, .. }
            | Member::Field { visibility, .. }
            | Member::Method { visibility, .. }
            | Member::Embed { visibility, .. } => visibility.unwrap_or(Visibility::Public),
            Member::StaticBlock { .. } | Member::ComptimeBlock { .. } | Member::Error { .. } => {
                Visibility::Public
            }
        };

        Some(MemberVisibilityContext {
            visibility,
            owner_symbol,
        })
    }

    /// Enforce visibility for a resolved member symbol.
    pub(crate) fn check_member_visibility(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        receiver_ty_id: LocalTypeId,
        state: &InferState,
    ) -> AnalyzeResult<()> {
        let Some(context) =
            self.member_visibility_context_for_symbol(ctx.tree_symbol_view(), member_symbol)?
        else {
            return Ok(());
        };

        self.check_visibility_context(
            ctx,
            expression_id,
            member_symbol,
            receiver_ty_id,
            state,
            context,
        );

        Ok(())
    }

    /// Enforce member visibility for a resolved visibility context.
    pub(crate) fn check_visibility_context(
        &self,
        ctx: &InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        receiver_ty_id: LocalTypeId,
        state: &InferState,
        context: MemberVisibilityContext,
    ) {
        // public members are always accessible
        if context.visibility == Visibility::Public {
            return;
        }

        // require a class/struct context for private and protected access
        let Some(current_class) = state.in_nominal_symbol else {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        };

        // private members require the declaring class
        if context.visibility == Visibility::Private && current_class != context.owner_symbol {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        }

        // protected members require a subclass context
        if context.visibility == Visibility::Protected
            && current_class != context.owner_symbol
            && !self.is_type_lineage_assignable(
                ctx.symbol_type_view(),
                current_class,
                context.owner_symbol,
            )
        {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        }

        // protected members must be accessed through the current class lineage
        if context.visibility == Visibility::Protected {
            let receiver_symbol = self.receiver_symbol_for_visibility(receiver_ty_id, ctx.types);
            if let Some(receiver_symbol) = receiver_symbol
                && receiver_symbol != current_class
                && !self.is_type_lineage_assignable(
                    ctx.symbol_type_view(),
                    receiver_symbol,
                    current_class,
                )
            {
                self.error(AnalyzeError::InaccessibleSymbol {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                    visibility: context.visibility,
                    symbol: member_symbol,
                });
            }
        }
    }

    /// Resolve visibility metadata for constructor parameter properties by key.
    pub(crate) fn parameter_property_member_context_for_key(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_symbol: GlobalSymbolId,
        member_key: &StaticKey,
    ) -> AnalyzeResult<Option<ParameterPropertyMemberContext>> {
        let StaticKey::Name(member_name) = member_key else {
            return Ok(None);
        };

        // walk the receiver lineage and find the first parameter property with this key
        let mut current_symbol = Some(receiver_symbol);
        while let Some(owner_symbol) = current_symbol {
            let context = self
                .with_module_tree_symbol_view_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    owner_symbol.module_id,
                    ctx.tree,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |view| {
                        let owner_entry = view.symbols.get_symbol(owner_symbol.local_id);
                        let declaration_id = owner_entry.primary_declaration?.local_id;
                        if declaration_id.ty != NodeType::Declaration {
                            return None;
                        }

                        let declaration = view.tree.get(declaration_id.into_typed::<Declaration>());
                        let members = declaration.member_ids()?;
                        for member_id in members {
                            let Member::Method {
                                signature,
                                is_static,
                                ..
                            } = view.tree.get(*member_id)
                            else {
                                continue;
                            };
                            if signature.mode != Some(FunctionMode::Constructor) {
                                continue;
                            }
                            if *is_static {
                                continue;
                            }

                            for parameter_id in &signature.parameters {
                                let parameter = view.tree.get(*parameter_id);
                                let Parameter::Named {
                                    name,
                                    visibility,
                                    is_readonly,
                                    ..
                                } = parameter
                                else {
                                    continue;
                                };
                                if name != member_name {
                                    continue;
                                }

                                let Some(visibility) = *visibility else {
                                    continue;
                                };

                                return Some(ParameterPropertyMemberContext {
                                    visibility,
                                    is_readonly: *is_readonly,
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
                .with_module_types_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    owner_symbol.module_id,
                    ctx.types,
                    destack_artifact::ArtifactKey::dir_declared,
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
        self.unwrap_type_value_symbol(types, receiver_ty_id)
    }
}
