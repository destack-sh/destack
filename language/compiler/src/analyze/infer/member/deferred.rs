use super::*;

impl Compiler {
    /// Report unresolved associated comptime projection obligations after infer convergence.
    pub(crate) fn report_associated_comptime_projection_obligation_errors(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<()> {
        let obligations = infer.take_associated_comptime_projection_obligations();
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        let mut reported = HashSet::new();

        let mut obligation_index = 0;
        while obligation_index < obligations.len() {
            let obligation = &obligations[obligation_index];
            let obligation_member_symbol = self.resolve_projection_obligation_member_symbol(
                module,
                profile,
                obligation.expression_id,
                obligation.member_symbol,
                &tree,
                &symbols,
                types,
            );
            let resolved_value_type_id = match self
                .resolved_projection_obligation_value_type_after_infer(
                    module,
                    profile,
                    obligation.expression_id.into_any(),
                    obligation_member_symbol,
                    &obligation.substitutions,
                    &tree,
                    &symbols,
                    types,
                ) {
                Ok(value) => value,
                Err(AnalyzeError::Yield { dependency }) => {
                    for obligation in obligations {
                        infer.push_associated_comptime_projection_obligation(obligation);
                    }

                    return Err(AnalyzeError::Yield { dependency });
                }
                Err(error) => return Err(error),
            };

            if let Some(value_type_id) = resolved_value_type_id {
                types.set_inferred_type(
                    obligation.expression_id.into_global_any(module.id),
                    value_type_id,
                );
                obligation_index += 1;
                continue;
            }
            if !reported.insert(obligation.expression_id.id) {
                obligation_index += 1;
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

            obligation_index += 1;
        }

        Ok(())
    }

    /// Return true when one projection needs an associated comptime obligation.
    pub(crate) fn projection_requires_associated_comptime_obligation(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        receiver_has_static_arguments: bool,
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
        if matches!(
            kind,
            Some(crate::analyze::StaticMemberSymbolKind::AssociatedComptimeConst)
        ) {
            return Ok(true);
        }

        // keep projection obligations for static-argument member accesses when kind facts are not available yet
        if kind.is_none() && receiver_has_static_arguments {
            return Ok(true);
        }

        Ok(false)
    }

    /// Resolve one projection obligation value type after infer convergence.
    /// Return None when the obligation remains unresolved.
    pub(crate) fn resolved_projection_obligation_value_type_after_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeIdAny,
        member_symbol: Option<GlobalSymbolId>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(member_symbol) = member_symbol else {
            return Ok(None);
        };
        if self.projection_obligation_substitutions_are_unresolved(
            module,
            profile,
            substitutions,
            symbols,
            types,
        ) {
            if !self.unresolved_projection_obligation_requires_primary_static_error_check(
                module,
                profile,
                member_symbol,
                symbols,
                types,
            )? {
                return Ok(None);
            }

            return self.projection_obligation_error_type_for_unresolved_substitutions_after_infer(
                module,
                profile,
                expression_id,
                member_symbol,
                substitutions,
                tree,
                symbols,
                types,
            );
        }

        self.resolved_projection_obligation_static_value_type_after_infer(
            module,
            profile,
            expression_id,
            member_symbol,
            substitutions,
            tree,
            symbols,
            types,
        )
    }

    /// Return true when unresolved substitutions may still produce a primary static-cycle error.
    #[allow(clippy::too_many_arguments)]
    fn unresolved_projection_obligation_requires_primary_static_error_check(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<bool> {
        self.query_symbol_has_associated_comptime_projection_dependencies(
            module,
            profile,
            member_symbol,
            symbols,
            types,
        )
    }

    /// Resolve one associated-comptime member symbol for one deferred projection obligation.
    #[allow(clippy::too_many_arguments)]
    fn resolve_projection_obligation_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<GlobalSymbolId> {
        if member_symbol.is_some() {
            return member_symbol;
        }

        let Expression::Member { left, name, .. } = tree.get(expression_id) else {
            return None;
        };
        let left = self.unwrap_parenthesized_expression(*left, tree);
        if !self.query_expression_is_projection_receiver_for_infer(
            module, profile, left, tree, symbols, types,
        ) {
            return None;
        }

        let selection = self
            .select_associated_projection_member_symbol(
                module,
                profile,
                expression_id,
                left,
                StaticKey::Name(*name),
                Some(StaticMemberSymbolKind::AssociatedComptimeConst),
                tree,
                symbols,
                types,
                true,
                true,
            )
            .ok()?;

        selection.map(|selection| selection.target_symbol)
    }

    /// Return true when one projection substitution environment remains unresolved.
    fn projection_obligation_substitutions_are_unresolved(
        &self,
        module: &Module,
        profile: ProfileId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        for substitution in substitutions.values().copied() {
            let substitution = types.unwrap_value_type_id(substitution);
            if self.type_contains_static_parameters(
                module,
                profile,
                substitution,
                symbols,
                types,
                &mut HashSet::new(),
            ) {
                return true;
            }
            if self.type_contains_infer_vars(substitution, types, &mut HashSet::new()) {
                return true;
            }
            if self.type_contains_unevaluated_static_arguments(
                substitution,
                types,
                &mut HashSet::new(),
            ) {
                return true;
            }
        }

        false
    }

    /// Resolve one projection obligation static value type after infer convergence.
    /// Return None when the static value is unresolved.
    #[allow(clippy::too_many_arguments)]
    fn projection_obligation_error_type_for_unresolved_substitutions_after_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(value_type_id) = self.projection_obligation_static_value_type_for_substitutions(
            module,
            profile,
            expression_id,
            member_symbol,
            substitutions,
            tree,
            symbols,
            types,
        )?
        else {
            return Ok(None);
        };

        let value_type_id = types.unwrap_value_type_id(value_type_id);
        if matches!(types.get_type(value_type_id), Type::Error) {
            return Ok(Some(value_type_id));
        }

        Ok(None)
    }

    /// Resolve one projection obligation static value type after infer convergence.
    /// Return None when the static value is unresolved.
    #[allow(clippy::too_many_arguments)]
    fn resolved_projection_obligation_static_value_type_after_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(value_type_id) = self.projection_obligation_static_value_type_for_substitutions(
            module,
            profile,
            expression_id,
            member_symbol,
            substitutions,
            tree,
            symbols,
            types,
        )?
        else {
            return Ok(None);
        };

        // keep error sentinels resolved here: primary diagnostics are emitted by static evaluation
        let committed_type_id = self.projection_obligation_committed_value_type_for_symbol(
            module,
            profile,
            expression_id,
            member_symbol,
            value_type_id,
            types,
        )?;

        Ok(Some(committed_type_id))
    }

    /// Resolve one projected static value type for one substitution environment.
    /// Return None when no static value can be produced yet.
    #[allow(clippy::too_many_arguments)]
    fn projection_obligation_static_value_type_for_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let mut visited_symbols = HashSet::new();
        let static_value = self.static_expression_from_constant_reference_specialized_declared(
            module,
            profile,
            member_symbol,
            tree,
            symbols,
            types,
            substitutions,
            &mut visited_symbols,
        )?;
        let Some(static_value) = static_value else {
            return Ok(None);
        };
        Ok(self.static_expression_type_id_for_substitution(expression_id, &static_value, types))
    }

    /// Resolve the committed value-space type for one projection obligation.
    fn projection_obligation_committed_value_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        fallback_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(value_type_id) = types.get_value_type_id(member_symbol) {
            return Ok(value_type_id);
        }
        if member_symbol.module_id != module.id {
            let imported_type_id = self.resolve_remote_symbol_value_type(
                module,
                profile,
                expression_id,
                member_symbol,
                true,
                types,
            )?;
            if !matches!(
                types.get_type(imported_type_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown
                }
            ) {
                return Ok(imported_type_id);
            }
        }

        // widen scalar literal projections in value position to preserve binding commit behavior
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(fallback_type_id)
        {
            let widened = Type::TypeLiteral {
                value: self.widen_scalar_literal_for_module(module, literal),
            };
            let widened_type_id = types.insert_type_from_any(widened, expression_id);
            return Ok(widened_type_id);
        }

        Ok(fallback_type_id)
    }
}
