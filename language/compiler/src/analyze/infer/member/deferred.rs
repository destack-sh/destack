use super::*;

impl Compiler {
    /// Report deferred unresolved associated comptime projection errors after infer convergence.
    pub(crate) fn report_deferred_associated_comptime_projection_errors(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) {
        let deferred = infer.take_deferred_associated_comptime_projections();
        let mut reported = HashSet::new();

        for projection in deferred {
            if !self.projection_is_unresolved_associated_comptime_value(
                module,
                profile,
                projection.receiver_id,
                projection.member_symbol,
                projection.member_type_id,
                tree,
                symbols,
                types,
            ) {
                continue;
            }
            if !reported.insert(projection.expression_id.id) {
                continue;
            }

            self.error(AnalyzeError::InvalidStaticArgument {
                node: projection
                    .expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                message: "associated comptime projection must be resolvable".to_string(),
            });

            let error_type_id = types.insert_type_from(Type::Error, projection.expression_id);
            types.set_inferred_type(
                projection.expression_id.into_global_any(module.id),
                error_type_id,
            );
        }
    }

    /// Return true when a static projection references an unresolved associated comptime value.
    pub(crate) fn projection_is_unresolved_associated_comptime_value(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        member_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        let Some(member_symbol) = member_symbol else {
            return false;
        };
        if !matches!(
            self.static_member_symbol_kind_for_symbol(
                module,
                profile,
                member_symbol,
                tree,
                symbols
            ),
            Some(crate::analyze::common::StaticMemberSymbolKind::AssociatedComptimeConst)
        ) {
            return false;
        }
        if !tree
            .get(receiver_id)
            .static_arguments()
            .is_some_and(|arguments| !arguments.is_empty())
        {
            return false;
        }

        matches!(
            types.get_type(member_type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown
            }
        )
    }

    /// Return a provisional type for unresolved associated comptime projections.
    pub(crate) fn provisional_type_for_unresolved_associated_comptime_projection(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: Option<GlobalSymbolId>,
        member_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let Some(member_symbol) = member_symbol else {
            return member_type_id;
        };

        if let Some(declared_member_type_id) = self.declared_member_type_for_symbol(
            module,
            profile,
            member_symbol,
            tree,
            symbols,
            types,
        ) {
            let declared_member_type_id = types.unwrap_value_type_id(declared_member_type_id);
            if !matches!(
                types.get_type(declared_member_type_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            ) {
                return declared_member_type_id;
            }
        }

        let Some(declared_member_type_id) = types.get_value_type_id(member_symbol) else {
            return member_type_id;
        };
        let declared_member_type_id = types.unwrap_value_type_id(declared_member_type_id);
        if matches!(
            types.get_type(declared_member_type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            }
        ) {
            return member_type_id;
        }

        declared_member_type_id
    }

    /// Resolve one declared type for a member symbol when available.
    pub(crate) fn declared_member_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let primary_declaration = self.with_module_tree_symbols_or_local(
            module,
            profile,
            member_symbol.module_id,
            tree,
            symbols,
            |_, _, owner_symbols| {
                owner_symbols
                    .get_symbol(member_symbol.local_id)
                    .primary_declaration
            },
        )?;

        self.declared_type_for_node(module, profile, primary_declaration, member_symbol, types)
    }
}
