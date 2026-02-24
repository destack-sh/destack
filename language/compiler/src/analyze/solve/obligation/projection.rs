use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    AssociatedComptimeProjectionObligation, Expression, GlobalSymbolId, InferTable, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, StaticKey, SymbolTable, SymbolType, Type, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

/// One deterministic action produced by projection obligation collection.
#[derive(Debug, Clone, Copy)]
enum ProjectionObligationAction {
    /// Set one inferred type for one projection expression.
    SetInferredType {
        /// The projection expression id.
        expression_id: LocalNodeId<Expression>,
        /// The inferred type id in the projection collection snapshot.
        type_id: LocalTypeId,
    },
    /// Emit one invalid projection diagnostic and set an error type.
    EmitInvalidProjection {
        /// The projection expression id.
        expression_id: LocalNodeId<Expression>,
    },
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Discharge projection obligations in solve and update infer overlays directly.
    pub(in crate::analyze::solve) fn discharge_projection_obligations_in_solve(
        &self,
        module: &Module,
        profile: ProfileId,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let mut obligations = infer.take_associated_comptime_projection_obligations();
        obligations.sort_by_key(|obligation| obligation.expression_id.id);
        if obligations.is_empty() {
            return Ok(());
        }

        let actions = match self.collect_projection_obligation_actions(
            module,
            profile,
            &obligations,
            types,
        ) {
            Ok(actions) => actions,
            Err(AnalyzeError::Yield { dependency }) => {
                for obligation in obligations {
                    infer.push_associated_comptime_projection_obligation(obligation);
                }
                return Err(AnalyzeError::Yield { dependency });
            }
            Err(error) => return Err(error),
        };

        self.apply_projection_obligation_actions_in_solve(module, profile, actions, infer, types)
    }

    /// Apply projection actions to infer overlays during solve discharge.
    fn apply_projection_obligation_actions_in_solve(
        &self,
        module: &Module,
        profile: ProfileId,
        actions: Vec<ProjectionObligationAction>,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        for action in actions {
            match action {
                ProjectionObligationAction::SetInferredType {
                    expression_id,
                    type_id,
                } => {
                    infer.set_inferred_type_for_node(
                        expression_id.into_global_any(module.id),
                        type_id,
                    );
                }
                ProjectionObligationAction::EmitInvalidProjection { expression_id } => {
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        message: "associated comptime projection must be resolvable".to_string(),
                    });

                    let error_type_id = types.insert_type_from(Type::Error, expression_id);
                    infer.set_inferred_type_for_node(
                        expression_id.into_global_any(module.id),
                        error_type_id,
                    );
                }
            }
        }

        Ok(())
    }

    /// Collect deterministic projection actions after infer convergence.
    fn collect_projection_obligation_actions(
        &self,
        module: &Module,
        profile: ProfileId,
        obligations: &[AssociatedComptimeProjectionObligation],
        types: &mut TypeTable,
    ) -> AnalyzeResult<Vec<ProjectionObligationAction>> {
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
            let resolved_value_type_id = self.resolve_projection_value_type(
                module,
                profile,
                obligation.expression_id.into_any(),
                obligation_member_symbol,
                &obligation.substitutions,
                &tree,
                &symbols,
                types,
            )?;

            // collect resolved projected value type actions
            if let Some(value_type_id) = resolved_value_type_id {
                if obligation_member_symbol.is_none() {
                    return Err(AnalyzeError::Internal {
                        message: "projection obligation resolved type without member symbol"
                            .to_string(),
                    });
                }
                actions.push(ProjectionObligationAction::SetInferredType {
                    expression_id: obligation.expression_id,
                    type_id: value_type_id,
                });
                continue;
            }

            // collect at most one unresolved projection diagnostic per expression id
            if reported.insert(obligation.expression_id.id) {
                actions.push(ProjectionObligationAction::EmitInvalidProjection {
                    expression_id: obligation.expression_id,
                });
            }
        }

        Ok(actions)
    }

    /// Resolve one projection obligation value type after infer convergence.
    /// Return None when the obligation remains unresolved.
    fn resolve_projection_value_type(
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
            if !self.symbol_has_projection_dependencies(module, profile, member_symbol, types)? {
                return Ok(None);
            }

            return self.resolve_projection_error_type(
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

        self.resolve_projection_static_value_type(
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

    /// Resolve one associated comptime member symbol for one deferred projection obligation.
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
        let is_projection_receiver = self
            .solve_projection_receiver_expression_for_projection_obligation(
                module, profile, left, tree, symbols, types,
            );
        if !is_projection_receiver {
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

    /// Return true when one receiver should be treated as an associated projection in solve.
    fn solve_projection_receiver_expression_for_projection_obligation(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);
        if !tree
            .get(receiver_id)
            .static_arguments()
            .is_some_and(|arguments| !arguments.is_empty())
        {
            return false;
        }

        let symbol = self
            .resolve_direct_receiver_symbol_for_expression(
                module,
                receiver_id,
                profile,
                tree,
                symbols,
            )
            .or_else(|| {
                let receiver_type_id = self
                    .resolve_declared_type_expression(
                        module,
                        profile,
                        receiver_id,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )
                    .ok()?;
                self.query_type_like_receiver_symbol_for_type_id(receiver_type_id, types)
            });
        let Some(symbol) = symbol else {
            return false;
        };

        let symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let symbol = self
            .declaration_symbol_id(module, symbols, profile, symbol)
            .unwrap_or(symbol);
        matches!(
            symbol.ty(),
            SymbolType::Class
                | SymbolType::Struct
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::TypeAlias
                | SymbolType::Newtype
        )
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
        // unresolved substitutions block projection discharge
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
    fn resolve_projection_error_type(
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
        let Some(value_type_id) = self.evaluate_projection_static_value_type(
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

        // keep primary static errors as resolved discharge output
        if matches!(types.get_type(value_type_id), Type::Error) {
            return Ok(Some(value_type_id));
        }

        Ok(None)
    }

    /// Resolve one projection obligation static value type after infer convergence.
    /// Return None when the static value is unresolved.
    fn resolve_projection_static_value_type(
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
        let Some(value_type_id) = self.evaluate_projection_static_value_type(
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
        let value_space_type_id = self.resolve_projection_value_type_for_symbol(
            module,
            expression_id,
            member_symbol,
            value_type_id,
            types,
        )?;

        Ok(Some(value_space_type_id))
    }

    /// Resolve one projected static value type for one substitution environment.
    /// Return None when no static value can be produced yet.
    fn evaluate_projection_static_value_type(
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

        // convert static expression discharge result into a type id
        Ok(self.static_expression_type_id_for_substitution(expression_id, &static_value, types))
    }

    /// Resolve the value-space type for one projection obligation.
    fn resolve_projection_value_type_for_symbol(
        &self,
        module: &Module,
        expression_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        provisional_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer existing value type ids when already available
        if let Some(value_type_id) = types.get_value_type_id(member_symbol) {
            return Ok(value_type_id);
        }

        // projection obligations use the projection-evaluated value type directly:
        // this keeps solve ownership local and avoids remote value-space imports here
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
