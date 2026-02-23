use crate::analyze::StaticMemberSymbolKind;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    AssociatedComptimeProjectionObligation, Expression, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, StaticKey, SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

/// One deterministic commit action produced by projection obligation collection.
#[derive(Debug, Clone, Copy)]
pub(in crate::analyze::commit) enum ProjectionCommitAction {
    /// Set one inferred type for one projection expression.
    SetInferredType {
        /// The projection expression id.
        expression_id: LocalNodeId<Expression>,
        /// The inferred type id in the projection collection snapshot.
        type_id: LocalTypeId,
    },
    /// Emit one invalid projection diagnostic and commit an error type.
    EmitInvalidProjection {
        /// The projection expression id.
        expression_id: LocalNodeId<Expression>,
    },
}

impl Compiler {
    /// Apply projection commit actions to committed type tables.
    pub(in crate::analyze::commit) fn apply_projection_commit_actions(
        &self,
        module: &Module,
        profile: ProfileId,
        actions: Vec<ProjectionCommitAction>,
        snapshot_types: &TypeTable,
        committed_types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let committed_type_floor = committed_types.type_count();
        let mut is_synchronized = false;
        for action in actions {
            match action {
                ProjectionCommitAction::SetInferredType {
                    expression_id,
                    type_id,
                } => {
                    let committed_type_id = self.committed_type_id_for_snapshot_type(
                        type_id,
                        snapshot_types,
                        committed_types,
                        committed_type_floor,
                        &mut is_synchronized,
                    )?;
                    committed_types.set_inferred_type(
                        expression_id.into_global_any(module.id),
                        committed_type_id,
                    );
                }
                ProjectionCommitAction::EmitInvalidProjection { expression_id } => {
                    // synchronize snapshot tail before local error type insertions
                    self.synchronize_snapshot_type_tail_for_commit(
                        snapshot_types,
                        committed_types,
                        committed_type_floor,
                        &mut is_synchronized,
                    )?;

                    // emit unresolved projection diagnostics
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        message: "associated comptime projection must be resolvable".to_string(),
                    });

                    // commit error type after reporting
                    let error_type_id =
                        committed_types.insert_type_from(Type::Error, expression_id);
                    committed_types
                        .set_inferred_type(expression_id.into_global_any(module.id), error_type_id);
                }
            }
        }

        Ok(())
    }

    /// Collect deterministic projection commit actions after infer convergence.
    pub(in crate::analyze::commit) fn collect_projection_commit_actions(
        &self,
        module: &Module,
        profile: ProfileId,
        obligations: &[AssociatedComptimeProjectionObligation],
        types: &mut TypeTable,
    ) -> AnalyzeResult<Vec<ProjectionCommitAction>> {
        // read module owned semantic tables once
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        let mut reported = HashSet::new();
        let mut actions = Vec::with_capacity(obligations.len());

        // resolve one action for each deferred projection obligation
        for obligation in obligations {
            // resolve or recover the projected member symbol
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
            let resolved_value_type_id = self
                .resolved_projection_obligation_value_type_after_infer(
                    module,
                    profile,
                    obligation.expression_id.into_any(),
                    obligation_member_symbol,
                    &obligation.substitutions,
                    &tree,
                    &symbols,
                    types,
                )?;

            // collect resolved projected value type commits
            if let Some(value_type_id) = resolved_value_type_id {
                if obligation_member_symbol.is_none() {
                    return Err(AnalyzeError::Internal {
                        message: "projection obligation resolved type without member symbol"
                            .to_string(),
                    });
                }
                actions.push(ProjectionCommitAction::SetInferredType {
                    expression_id: obligation.expression_id,
                    type_id: value_type_id,
                });
                continue;
            }

            // collect at most one unresolved projection diagnostic per expression id
            if reported.insert(obligation.expression_id.id) {
                actions.push(ProjectionCommitAction::EmitInvalidProjection {
                    expression_id: obligation.expression_id,
                });
            }
        }

        Ok(actions)
    }

    /// Resolve one projection obligation value type after infer convergence.
    /// Return None when the obligation remains unresolved.
    pub(super) fn resolved_projection_obligation_value_type_after_infer(
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
        let static_value = self.resolve_static_constant_reference_instantiated_declared(
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
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        provisional_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer committed value type ids when already available
        if let Some(value_type_id) = types.get_value_type_id(member_symbol) {
            return Ok(value_type_id);
        }

        // projection obligations commit the projection-evaluated value type directly:
        // this keeps commit ownership local and avoids remote value-space imports here
        Ok(self.widen_projection_value_type_for_value_position(
            module,
            expression_id,
            provisional_type_id,
            types,
        ))
    }

    /// Widen one projected scalar literal type for value position when required.
    fn widen_projection_value_type_for_value_position(
        &self,
        module: &Module,
        expression_id: LocalNodeIdAny,
        provisional_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(provisional_type_id)
        {
            let widened = Type::TypeLiteral {
                value: self.widen_scalar_literal_for_module(module, literal),
            };
            let widened_type_id = types.insert_type_from_any(widened, expression_id);
            return widened_type_id;
        }

        provisional_type_id
    }
}
