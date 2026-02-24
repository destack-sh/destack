use crate::analyze::common::RelationMode;
use crate::analyze::infer::member::MemberResolution;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{
    DispatchKey, Expression, GlobalSymbolId, InferTable, LocalNodeId, LocalTypeId,
    MissingMemberObligation, NormalizationMode, Resolution, ResolutionCandidate, StaticKey, Type,
    TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

/// One deferred resolution for one missing-member obligation.
#[derive(Debug, Clone)]
enum MissingMemberObligationResolution {
    /// One static member symbol resolved for the obligation expression.
    Static {
        /// The selected member symbol.
        symbol: GlobalSymbolId,
    },
    /// One dynamic member dispatch list resolved for the obligation expression.
    Dynamic {
        /// The dynamic dispatch candidates selected for this member access.
        candidates: Vec<MissingMemberObligationCandidate>,
    },
    /// No concrete member was selected, and lookup remains unresolved.
    Unresolved,
}

/// One dynamic member dispatch candidate resolved during missing-member obligation collection.
#[derive(Debug, Clone)]
struct MissingMemberObligationCandidate {
    /// The receiver type id used for this candidate in the collection snapshot.
    receiver_ty_id: LocalTypeId,
    /// The selected member symbol for this receiver type.
    symbol: GlobalSymbolId,
}

/// One deterministic action produced by missing-member obligation collection.
#[derive(Debug, Clone)]
enum MissingMemberObligationAction {
    /// Set one member lookup resolution for one expression.
    SetResolution {
        /// The member expression that owns this resolution.
        expression_id: LocalNodeId<Expression>,
        /// The receiver type id in the collection snapshot.
        receiver_ty_id: LocalTypeId,
        /// The collected resolution to apply.
        resolution: MissingMemberObligationResolution,
    },
    /// Emit one missing-member diagnostic and set an error type.
    EmitMissingMemberDiagnostic {
        /// The member expression that failed lookup.
        expression_id: LocalNodeId<Expression>,
        /// The receiver type id in the collection snapshot.
        receiver_ty_id: LocalTypeId,
        /// The missing member key.
        member_key: StaticKey,
    },
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Discharge missing-member obligations in solve and write infer overlays directly.
    pub(in crate::analyze::solve) fn discharge_missing_member_obligations_in_solve(
        &self,
        module: &Module,
        profile: ProfileId,
        infer: &mut InferTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        let mut obligations = infer.take_missing_member_obligations();
        obligations.sort_by_key(|obligation| {
            (obligation.expression_id, obligation.receiver_expression_id)
        });
        if obligations.is_empty() {
            return Ok(());
        }

        let actions = match self.collect_missing_member_obligation_actions(
            module,
            profile,
            &obligations,
            infer,
            types,
            options,
        ) {
            Ok(actions) => actions,
            Err(AnalyzeError::Yield { dependency }) => {
                for obligation in obligations {
                    infer.push_missing_member_obligation(obligation);
                }
                return Err(AnalyzeError::Yield { dependency });
            }
            Err(error) => return Err(error),
        };

        self.apply_missing_member_obligation_actions_in_solve(
            module, profile, actions, infer, types,
        )
    }

    /// Map one member-resolution result to obligation resolution form.
    fn resolve_missing_member_obligation_resolution(
        &self,
        resolution: &MemberResolution,
    ) -> Option<MissingMemberObligationResolution> {
        match resolution {
            MemberResolution::Static { symbol } => {
                Some(MissingMemberObligationResolution::Static { symbol: *symbol })
            }
            MemberResolution::Dynamic { candidates } => {
                let candidates = candidates
                    .iter()
                    .map(|candidate| MissingMemberObligationCandidate {
                        receiver_ty_id: candidate.receiver_ty_id,
                        symbol: candidate.symbol,
                    })
                    .collect::<Vec<_>>();
                Some(MissingMemberObligationResolution::Dynamic { candidates })
            }
            MemberResolution::None | MemberResolution::Unresolved => None,
        }
    }

    /// Collect missing-member obligation actions from deferred obligations.
    fn collect_missing_member_obligation_actions(
        &self,
        module: &Module,
        profile: ProfileId,
        obligations: &[MissingMemberObligation],
        infer: &InferTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<Vec<MissingMemberObligationAction>> {
        // read module owned semantic tables once
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        let mut reported = HashSet::new();
        let mut actions = Vec::with_capacity(obligations.len());

        // discharge each obligation after solver convergence
        for obligation in obligations {
            // skip obligations attached to other modules
            if obligation.expression_id.module_id != module.id
                || obligation.receiver_expression_id.module_id != module.id
            {
                continue;
            }

            // decode expression ids from the global obligation record
            let expression_id = obligation
                .expression_id
                .local_id
                .try_into_typed::<Expression>()
                .map_err(|_| AnalyzeError::Internal {
                    message: "missing member obligation expression is not an expression"
                        .to_string(),
                })?;
            let receiver_expression_id = obligation
                .receiver_expression_id
                .local_id
                .try_into_typed::<Expression>()
                .map_err(|_| AnalyzeError::Internal {
                    message: "missing member obligation receiver is not an expression".to_string(),
                })?;

            // skip stale obligations once the expression already has a concrete inferred type
            if let Some(current_expression_ty_id) = infer
                .inferred_type_for_node(obligation.expression_id)
                .or_else(|| types.get_inferred_type_id(obligation.expression_id))
            {
                let current_expression_ty_id = types.unwrap_value_type_id(current_expression_ty_id);
                if !matches!(
                    types.get_type(current_expression_ty_id),
                    Type::InferVar { .. }
                        | Type::Unevaluated(_)
                        | Type::Error
                        | Type::TypeLiteral {
                            value: TypeLiteral::Unknown
                        }
                ) {
                    continue;
                }
            }

            // recover and normalize the receiver type for discharged lookup
            let mut receiver_ty_id = infer
                .inferred_type_for_node(obligation.receiver_expression_id)
                .or_else(|| types.get_inferred_type_id(obligation.receiver_expression_id))
                .unwrap_or(obligation.receiver_type_id);
            if self.type_is_solver_placeholder(receiver_ty_id, types)
                && let Some(receiver_symbol) = tree.get(receiver_expression_id).target_symbol()
                && let Some(symbol_type_id) =
                    types.get_type_id_for_symbol(&symbols, receiver_symbol)
            {
                receiver_ty_id = symbol_type_id;
            }
            let receiver_ty_id = self.materialize_infer_type_for_check(
                module,
                profile,
                &symbols,
                receiver_ty_id,
                infer,
                types,
                options,
            );
            let receiver_ty_id = self.normalize_apparent_type(
                module,
                profile,
                receiver_ty_id,
                &symbols,
                types,
                NormalizationMode::Assign,
                RelationMode::ASSIGN,
            );
            if self.type_is_solver_placeholder(receiver_ty_id, types) {
                continue;
            }

            // suppress cascades after primary receiver failure
            if self.type_blocks_cascading_diagnostic(receiver_ty_id, types) {
                continue;
            }

            // fail closed when discharge still depends on unsolved state
            if self.type_relation_requires_infer_convergence(
                module,
                profile,
                receiver_ty_id,
                receiver_ty_id,
                &symbols,
                types,
            ) {
                if self.type_blocks_cascading_diagnostic(receiver_ty_id, types) {
                    continue;
                }

                return Err(AnalyzeError::Internal {
                    message: "unresolved missing member obligation after infer convergence"
                        .to_string(),
                });
            }

            // resolve member dispatch for the converged receiver
            let receiver_ty = types.get_type(receiver_ty_id).clone();
            let receiver_context = self.query_member_receiver_context_for_expression(
                module,
                receiver_expression_id,
                Some(receiver_ty_id),
                profile,
                &tree,
                &symbols,
                types,
            );
            let member_resolution = self.resolve_member_symbol_for_receiver(
                module,
                expression_id,
                receiver_expression_id,
                &receiver_ty,
                &receiver_context,
                &obligation.member_key,
                profile,
                &tree,
                &symbols,
                types,
            )?;
            if let Some(resolution) =
                self.resolve_missing_member_obligation_resolution(&member_resolution)
            {
                actions.push(MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolution,
                });
                continue;
            }

            // resolve index signature value access path
            let mut index_visited = Vec::new();
            let index_type_id = self.resolve_index_signature_value_type_for_key(
                module,
                profile,
                expression_id.into_any(),
                &symbols,
                &receiver_ty,
                &obligation.member_key,
                types,
                &mut index_visited,
            );
            if index_type_id.is_some() {
                actions.push(MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolution: MissingMemberObligationResolution::Unresolved,
                });
                continue;
            }

            // emit at most one diagnostic per expression id
            if !reported.insert(expression_id.id) {
                continue;
            }

            // report missing member diagnostics for unresolved lookups
            let allow_associated_contract_blocker = self.is_projection_receiver_expression(
                module,
                profile,
                receiver_expression_id,
                &tree,
                &symbols,
                types,
            );
            let blocker = self.should_block_missing_member_diagnostic(
                module,
                profile,
                receiver_ty_id,
                &symbols,
                types,
                allow_associated_contract_blocker,
            )?;
            if blocker.is_some() {
                continue;
            }

            actions.push(MissingMemberObligationAction::EmitMissingMemberDiagnostic {
                expression_id,
                receiver_ty_id,
                member_key: obligation.member_key.clone(),
            });
        }

        Ok(actions)
    }

    /// Apply missing-member obligation actions in solve.
    fn apply_missing_member_obligation_actions_in_solve(
        &self,
        module: &Module,
        profile: ProfileId,
        actions: Vec<MissingMemberObligationAction>,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        for action in actions {
            match action {
                MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolution,
                } => {
                    let receiver_ty_id = Some(receiver_ty_id);
                    let resolution = match resolution {
                        MissingMemberObligationResolution::Static { symbol } => {
                            let candidate = ResolutionCandidate {
                                key: None,
                                target_symbol: symbol,
                                instance: None,
                                resolved_signature: None,
                            };
                            Resolution::Static {
                                receiver: receiver_ty_id,
                                candidate,
                            }
                        }
                        MissingMemberObligationResolution::Dynamic { candidates } => {
                            let candidates = candidates
                                .into_iter()
                                .map(|candidate| -> AnalyzeResult<ResolutionCandidate> {
                                    Ok(ResolutionCandidate {
                                        key: Some(DispatchKey::single(candidate.receiver_ty_id)),
                                        target_symbol: candidate.symbol,
                                        instance: None,
                                        resolved_signature: None,
                                    })
                                })
                                .collect::<AnalyzeResult<Vec<_>>>()?;
                            Resolution::Dynamic {
                                receiver: receiver_ty_id,
                                candidates,
                            }
                        }
                        MissingMemberObligationResolution::Unresolved => Resolution::Unresolved {
                            receiver: receiver_ty_id,
                            missing_keys: Vec::new(),
                            candidates: Vec::new(),
                        },
                    };
                    let expression_global = expression_id.into_global_any(module.id);
                    infer.set_provisional_resolution_for_node(expression_global, resolution);
                }
                MissingMemberObligationAction::EmitMissingMemberDiagnostic {
                    expression_id,
                    receiver_ty_id,
                    member_key,
                } => {
                    self.error(AnalyzeError::MissingMember {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        receiver_ty: receiver_ty_id.into_global(module.id),
                        member_key,
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
}
