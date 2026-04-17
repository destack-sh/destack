use super::*;
use crate::analyze::common::{InferContext, ModuleSymbolView, SymbolTypeView, TreeSymbolView};

impl Compiler {
    /// Resolve the enum symbol that owns an enum field symbol.
    pub(crate) fn enum_symbol_for_enum_field_symbol(
        &self,
        view: ModuleSymbolView<'_>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve the enum field symbol entry
        let Some((is_enum_field, scope_owner)) = self
            .with_module_symbols_or_local_for_artifact(
                view.compiler_context,
                view.module,
                view.profile,
                member_symbol.module_id,
                view.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |_, owner_symbols| {
                    let member_entry = owner_symbols.get_symbol(member_symbol.local_id);
                    let scope = owner_symbols.get_scope_by_symbol(member_symbol.local_id);
                    let is_enum_field = member_entry
                        .primary_declaration
                        .is_some_and(|declaration| declaration.local_id.ty == NodeType::EnumField);
                    scope.owner_id.map(|owner_id| (is_enum_field, owner_id))
                },
            )
            .map_err(AnalyzeError::from)?
        else {
            return Ok(None);
        };

        // ensure the symbol is an enum field
        if !is_enum_field {
            return Ok(None);
        }

        // ensure the owning symbol is an enum
        if scope_owner.ty != SymbolType::Enum {
            return Ok(None);
        }

        Ok(Some(scope_owner.into_global(member_symbol.module_id)))
    }

    /// Resolve enum field member symbols when the receiver is an enum reference.
    pub(crate) fn resolve_enum_field_member_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        left_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
        member_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // keep already resolved member symbols
        if member_symbol.is_some() {
            return Ok(member_symbol);
        }

        // only direct enum references can resolve enum field symbols
        let Some(left_symbol) =
            self.reference_symbol_for_expression(ctx.tree_symbol_view(), left_id)
        else {
            return Ok(None);
        };
        if left_symbol.ty() != SymbolType::Enum {
            return Ok(None);
        }

        // resolve the member symbol from the enum declaration
        let mut visited = Vec::new();
        self.resolve_member_symbol_for_symbol(
            ctx.compiler_context,
            ctx.module,
            ctx.module.id,
            ctx.profile,
            &ctx.index,
            ctx.tree,
            ctx.symbols,
            ctx.types,
            left_symbol,
            member_key,
            MemberLookupMode::Value,
            &mut visited,
        )
    }

    /// Resolve enum field member access when the receiver is an enum.
    pub(crate) fn resolve_enum_field_access(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        left_ty_id: LocalTypeId,
        left_ty: &Type,
        member_key: &StaticKey,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // select the enum symbol for the receiver
        let enum_symbol = self
            .enum_symbol_for_receiver_symbol(ctx.tree_symbol_view(), left_id)
            .or_else(|| self.enum_symbol_for_type(left_ty, ctx.types));
        let Some(enum_symbol) = enum_symbol else {
            return Ok(None);
        };

        // resolve the enum field symbol for the requested member key
        let enum_field_symbol =
            self.enum_field_symbol_for_member_key(ctx.tree_symbol_view(), enum_symbol, member_key)?;
        let Some(enum_field_symbol) = enum_field_symbol else {
            return Ok(None);
        };

        // record the member resolution for the enum field
        let resolution = MemberResolution::Static {
            symbol: enum_field_symbol,
        };
        self.record_provisional_member_resolution(
            expression_id.into_global_any(ctx.module.id),
            Some(left_ty_id),
            &resolution,
            None,
            None,
            true,
            ctx.infer,
            ctx.types,
        );

        // return the nominal enum reference type
        let enum_reference = Type::Reference {
            symbol: enum_symbol,
            generic_arguments: None,
        };
        Ok(Some(
            ctx.types.insert_type_from(enum_reference, expression_id),
        ))
    }

    /// Resolve the value type for an enum field symbol when possible.
    pub(crate) fn enum_field_value_type_for_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        member_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(member_symbol) = member_symbol else {
            return Ok(None);
        };

        if self
            .enum_symbol_for_enum_field_symbol(ctx.module_symbol_view(), member_symbol)?
            .is_none()
        {
            return Ok(None);
        }

        Ok(ctx.types.get_value_type_id(member_symbol))
    }

    /// Resolve enum symbols from a receiver expression when it is a direct reference.
    pub(crate) fn enum_symbol_for_receiver_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        left_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        let left_symbol = self.reference_symbol_for_expression(ctx, left_id)?;
        if left_symbol.ty() == SymbolType::Enum {
            Some(left_symbol)
        } else {
            None
        }
    }

    /// Resolve an enum field symbol matching a member key.
    pub(crate) fn enum_field_symbol_for_member_key(
        &self,
        ctx: TreeSymbolView<'_>,
        enum_symbol: GlobalSymbolId,
        member_key: &StaticKey,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve local declarations when possible
        if enum_symbol.module_id == ctx.module.id {
            return Ok(self.enum_field_symbol_for_member_key_in_tree(ctx, enum_symbol, member_key));
        }

        let module = ctx.compiler_context.module(enum_symbol.module_id);
        let module = module.as_ref();
        let dir = self
            .require_artifact_dir_declared(
                ctx.compiler_context.revision(),
                enum_symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;

        Ok(self.enum_field_symbol_for_member_key_in_tree(
            TreeSymbolView::new(
                ctx.compiler_context,
                module,
                ctx.profile,
                &dir.tree,
                &dir.symbols,
            ),
            enum_symbol,
            member_key,
        ))
    }

    /// Resolve enum declarations for a matching field key.
    pub(crate) fn enum_field_symbol_for_member_key_in_tree(
        &self,
        ctx: TreeSymbolView<'_>,
        enum_symbol: GlobalSymbolId,
        member_key: &StaticKey,
    ) -> Option<GlobalSymbolId> {
        // ensure we are scanning an enum symbol
        let symbol_entry = ctx.symbols.get_symbol(enum_symbol.local_id);
        if symbol_entry.ty != SymbolType::Enum {
            return None;
        }

        // collect the enum declarations for the symbol
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        // scan enum fields for a matching key
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let Declaration::Enum(declaration) = ctx.tree.get(declaration_id) else {
                continue;
            };
            for field_id in &declaration.fields {
                let field = ctx.tree.get(*field_id);
                let field_key = StaticKey::Name(field.name.string());
                if field_key.matches(member_key) {
                    return self.enum_field_symbol_for_name_in_tree(
                        ctx,
                        enum_symbol,
                        field.name.string(),
                    );
                }
            }
        }

        None
    }
}
