use crate::analyze::StaticMemberSymbolKind;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, InferTable, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree,
    StaticKey, SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

impl Compiler {
    /// Report unresolved associated comptime projection obligations after infer convergence.
    pub(crate) fn report_associated_comptime_projection_obligation_errors(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<()> {
        // collect deferred projection obligations from infer state
        let obligations = infer.take_associated_comptime_projection_obligations();

        // read module owned semantic tables once
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        let mut reported = HashSet::new();

        // replay each deferred projection obligation
        let mut obligation_index = 0;
        while obligation_index < obligations.len() {
            // resolve or recover the projected member symbol
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

            // resolve projected value type for the converged substitution environment
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
                    // restore obligations when replay yields on dependencies
                    for obligation in obligations {
                        infer.push_associated_comptime_projection_obligation(obligation);
                    }

                    return Err(AnalyzeError::Yield { dependency });
                }
                Err(error) => return Err(error),
            };

            // commit resolved projected value types
            if let Some(value_type_id) = resolved_value_type_id {
                types.set_inferred_type(
                    obligation.expression_id.into_global_any(module.id),
                    value_type_id,
                );
                obligation_index += 1;
                continue;
            }

            // report one diagnostic per expression id
            if !reported.insert(obligation.expression_id.id) {
                obligation_index += 1;
                continue;
            }

            // emit unresolved projection diagnostics
            self.error(AnalyzeError::InvalidStaticArgument {
                node: obligation
                    .expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                message: "associated comptime projection must be resolvable".to_string(),
            });

            // commit error type after reporting
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
        // check member kind facts when available
        let kind = self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                member_symbol,
                tree,
                symbols,
            )
            .map_err(AnalyzeError::from)?;
        if matches!(kind, Some(StaticMemberSymbolKind::AssociatedComptimeConst)) {
            return Ok(true);
        }

        // keep obligations when kind facts are not available yet
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
        // unresolved member symbols stay deferred
        let Some(member_symbol) = member_symbol else {
            return Ok(None);
        };

        // unresolved substitutions may still lead to primary static cycle errors
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

    /// Return true when unresolved substitutions may still produce a primary static cycle error.
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

    /// Resolve one associated comptime member symbol for one deferred projection obligation.
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
        // keep eager member symbols from infer
        if member_symbol.is_some() {
            return member_symbol;
        }

        // only member expressions can recover projection symbols
        let Expression::Member { left, name, .. } = tree.get(expression_id) else {
            return None;
        };
        let left = self.unwrap_parenthesized_expression(*left, tree);

        // recover only for projection receivers
        if !self.query_expression_is_projection_receiver_for_infer(
            module, profile, left, tree, symbols, types,
        ) {
            return None;
        }

        // select associated comptime members from projection syntax
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
        // unresolved substitutions block projection replay
        for substitution in substitutions.values().copied() {
            let substitution = types.unwrap_value_type_id(substitution);
            if !self.type_is_converged_for_static_evaluation(
                module,
                profile,
                substitution,
                symbols,
                types,
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
        // resolve a projected static value type for unresolved substitutions
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

        // keep primary static errors as resolved replay output
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
        // resolve projected static value type for converged substitutions
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

        // keep error sentinels resolved here: static evaluation reports the primary diagnostic
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
        // evaluate static expressions under captured substitutions
        let mut visited_symbols = HashSet::new();
        let static_value = self.static_expression_from_constant_reference_instantiated_declared(
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

        // convert static expression replay result into a type id
        Ok(self.static_expression_type_id_for_substitution(expression_id, &static_value, types))
    }

    /// Resolve the committed value space type for one projection obligation.
    fn projection_obligation_committed_value_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        provisional_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer committed value type ids when already available
        if let Some(value_type_id) = types.get_value_type_id(member_symbol) {
            return Ok(value_type_id);
        }

        // import value type ids for remote symbols
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

        // widen scalar literal projections in value position for binding commits
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(provisional_type_id)
        {
            let widened = Type::TypeLiteral {
                value: self.widen_scalar_literal_for_module(module, literal),
            };
            let widened_type_id = types.insert_type_from_any(widened, expression_id);
            return Ok(widened_type_id);
        }

        Ok(provisional_type_id)
    }
}
