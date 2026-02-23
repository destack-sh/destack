use super::*;

impl Compiler {
    /// Resolve the enum symbol that owns an enum field symbol.
    pub(crate) fn enum_symbol_for_enum_field_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve the enum field symbol entry
        let Some((is_enum_field, scope_owner)) = self
            .with_module_symbols_base_or_local_at_stage(
                module,
                profile,
                member_symbol.module_id,
                symbols,
                AnalyzeDependencyStage::Declare,
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
        module: &Module,
        left_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
        member_symbol: Option<GlobalSymbolId>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // keep already resolved member symbols
        if member_symbol.is_some() {
            return Ok(member_symbol);
        }

        // only direct enum references can resolve enum field symbols
        let Some(left_symbol) =
            self.reference_symbol_for_expression(module, left_id, profile, tree, symbols)
        else {
            return Ok(None);
        };
        if left_symbol.ty() != SymbolType::Enum {
            return Ok(None);
        }

        // resolve the member symbol from the enum declaration
        let mut visited = Vec::new();
        self.resolve_member_symbol_for_symbol(
            module,
            left_symbol,
            member_key,
            MemberLookupMode::Value,
            profile,
            tree,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Resolve enum field member access when the receiver is an enum.
    pub(crate) fn resolve_enum_field_access(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        left_ty_id: LocalTypeId,
        left_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // select the enum symbol for the receiver
        let enum_symbol = self
            .enum_symbol_for_receiver_symbol(module, left_id, profile, tree, symbols)
            .or_else(|| self.enum_symbol_for_type(left_ty, types));
        let Some(enum_symbol) = enum_symbol else {
            return Ok(None);
        };

        // resolve the enum field symbol for the requested member key
        let enum_field_symbol = self.enum_field_symbol_for_member_key(
            module,
            enum_symbol,
            member_key,
            profile,
            tree,
            symbols,
        )?;
        let Some(enum_field_symbol) = enum_field_symbol else {
            return Ok(None);
        };

        // record the member resolution for the enum field
        let resolution = MemberResolution::Static {
            symbol: enum_field_symbol,
        };
        self.record_provisional_member_resolution(
            expression_id.into_global_any(module.id),
            Some(left_ty_id),
            &resolution,
            None,
            None,
            true,
            infer,
            types,
        );

        // return the nominal enum reference type
        let enum_reference = Type::Reference {
            symbol: enum_symbol,
            static_arguments: None,
        };
        Ok(Some(types.insert_type_from(enum_reference, expression_id)))
    }

    /// Resolve the value type for an enum field symbol when possible.
    pub(crate) fn enum_field_value_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        member_symbol: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(member_symbol) = member_symbol else {
            return Ok(None);
        };

        if self
            .enum_symbol_for_enum_field_symbol(module, profile, symbols, member_symbol)?
            .is_none()
        {
            return Ok(None);
        }

        Ok(types.get_value_type_id(member_symbol))
    }

    /// Resolve enum symbols from a receiver expression when it is a direct reference.
    pub(crate) fn enum_symbol_for_receiver_symbol(
        &self,
        module: &Module,
        left_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let left_symbol =
            self.reference_symbol_for_expression(module, left_id, profile, tree, symbols)?;
        if left_symbol.ty() == SymbolType::Enum {
            Some(left_symbol)
        } else {
            None
        }
    }

    /// Resolve an enum field symbol matching a member key.
    pub(crate) fn enum_field_symbol_for_member_key(
        &self,
        module: &Module,
        enum_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve local declarations when possible
        if enum_symbol.module_id == module.id {
            return Ok(self.enum_field_symbol_for_member_key_in_tree(
                enum_symbol,
                member_key,
                tree,
                symbols,
            ));
        }

        self.with_module_tree_symbols_at_stage(
            module,
            profile,
            enum_symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |_, owner_tree, owner_symbols| {
                self.enum_field_symbol_for_member_key_in_tree(
                    enum_symbol,
                    member_key,
                    owner_tree,
                    owner_symbols,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve enum declarations for a matching field key.
    pub(crate) fn enum_field_symbol_for_member_key_in_tree(
        &self,
        enum_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // ensure we are scanning an enum symbol
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
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
            let Declaration::Enum { fields, .. } = tree.get(declaration_id) else {
                continue;
            };
            for field_id in fields {
                let field = tree.get(*field_id);
                let field_key = StaticKey::Name(field.name);
                if field_key.matches(member_key) {
                    return Some(field.symbol.into_global(enum_symbol.module_id));
                }
            }
        }

        None
    }
}
