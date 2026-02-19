use super::*;

impl Compiler {
    /// Report unresolved associated comptime projection obligations after infer convergence.
    pub(crate) fn report_associated_comptime_projection_obligation_errors(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) {
        let obligations = infer.take_associated_comptime_projection_obligations();
        let symbols = module.dir(profile).symbols.read();
        let mut reported = HashSet::new();

        for obligation in obligations {
            if !self.projection_obligation_is_unresolved(
                module,
                profile,
                obligation.member_type_id,
                &obligation.substitutions,
                &symbols,
                types,
            ) {
                continue;
            }
            if !reported.insert(obligation.expression_id.id) {
                continue;
            }

            self.error(AnalyzeError::InvalidStaticArgument {
                node: obligation
                    .expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                message: "associated comptime projection must be resolvable".to_string(),
            });

            let error_type_id = types.insert_type_from(Type::Error, obligation.expression_id);
            types.set_inferred_type(
                obligation.expression_id.into_global_any(module.id),
                error_type_id,
            );
        }
    }

    /// Return true when one projection needs an associated comptime obligation.
    pub(crate) fn projection_requires_associated_comptime_obligation(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        _receiver_has_static_arguments: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<bool> {
        let kind = self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                member_symbol,
                tree,
                symbols,
            )
            .map_err(AnalyzeError::from)?;
        Ok(matches!(
            kind,
            Some(crate::analyze::common::StaticMemberSymbolKind::AssociatedComptimeConst)
        ))
    }

    /// Return true when one obligation type is still unresolved after infer convergence.
    pub(crate) fn projection_obligation_type_is_unresolved(
        &self,
        member_type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        matches!(
            types.get_type(member_type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown
            }
        )
    }

    /// Return true when one projection obligation still depends on unsolved substitutions.
    pub(crate) fn projection_obligation_is_unresolved(
        &self,
        module: &Module,
        profile: ProfileId,
        member_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        if self.projection_obligation_type_is_unresolved(member_type_id, types) {
            return true;
        }

        for type_id in substitutions.values().copied() {
            if self.type_contains_static_parameters(
                module,
                profile,
                type_id,
                symbols,
                types,
                &mut HashSet::new(),
            ) {
                return true;
            }
            if self.type_contains_infer_vars(type_id, types, &mut HashSet::new()) {
                return true;
            }
            if self.type_contains_unevaluated_static_arguments(type_id, types, &mut HashSet::new())
            {
                return true;
            }
        }

        false
    }
}
