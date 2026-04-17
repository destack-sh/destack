use crate::analyze::common::{CanonicalSymbolMode, InferContext, RelationMode, TypeContext};
use crate::analyze::infer::member::MemberResolution;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DispatchKey, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, MissingMemberObligation,
    NormalizationMode, Resolution, ResolutionCandidate, StaticKey, SymbolType, Type, TypeLiteral,
};
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
        /// The resolved member value type when available.
        resolved_member_type_id: Option<LocalTypeId>,
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
        ctx: &mut InferContext<'_>,
    ) -> AnalyzeResult<()> {
        let mut obligations = ctx.infer.take_missing_member_obligations();
        obligations.sort_by_key(|obligation| {
            (obligation.expression_id, obligation.receiver_expression_id)
        });
        if obligations.is_empty() {
            return Ok(());
        }

        let actions = match self.query_missing_member_obligation_actions(ctx, &obligations)? {
            Some(actions) => actions,
            None => {
                for obligation in obligations {
                    ctx.infer.push_missing_member_obligation(obligation);
                }
                return Ok(());
            }
        };

        self.apply_missing_member_obligation_actions_in_solve(ctx, actions)
    }

    /// Query missing-member obligation actions after infer convergence.
    fn query_missing_member_obligation_actions(
        &self,
        ctx: &mut InferContext<'_>,
        obligations: &[MissingMemberObligation],
    ) -> AnalyzeResult<Option<Vec<MissingMemberObligationAction>>> {
        match self.collect_missing_member_obligation_actions(ctx, obligations) {
            Ok(actions) => Ok(Some(actions)),
            Err(AnalyzeError::Yield { .. }) => Ok(None),
            Err(error) => Err(error),
        }
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
        ctx: &mut InferContext<'_>,
        obligations: &[MissingMemberObligation],
    ) -> AnalyzeResult<Vec<MissingMemberObligationAction>> {
        let mut reported = HashSet::new();
        let mut actions = Vec::with_capacity(obligations.len());

        // discharge each obligation after solver convergence
        for obligation in obligations {
            // skip obligations attached to other modules
            if obligation.expression_id.module_id != ctx.module.id
                || obligation.receiver_expression_id.module_id != ctx.module.id
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
            if let Some(current_expression_ty_id) = ctx
                .infer
                .inferred_type_for_node(obligation.expression_id)
                .or_else(|| ctx.types.get_inferred_type_id(obligation.expression_id))
            {
                let current_expression_ty_id =
                    ctx.types.unwrap_value_type_id(current_expression_ty_id);
                if !matches!(
                    ctx.types.get_type(current_expression_ty_id),
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
            let mut receiver_ty_id = ctx
                .infer
                .inferred_type_for_node(obligation.receiver_expression_id)
                .or_else(|| {
                    ctx.types
                        .get_inferred_type_id(obligation.receiver_expression_id)
                })
                .unwrap_or(obligation.receiver_type_id);
            if self.type_is_solver_placeholder(receiver_ty_id, ctx.types)
                && let Some(receiver_symbol) = ctx.tree.get(receiver_expression_id).target_symbol()
                && let Some(symbol_type_id) = ctx
                    .types
                    .get_type_id_for_symbol(ctx.symbols, receiver_symbol)
            {
                receiver_ty_id = symbol_type_id;
            }
            let receiver_ty_id =
                self.materialize_infer_type_for_check(&mut ctx.reborrow(), receiver_ty_id);
            let receiver_ty_id = self.normalize_apparent_type(
                &mut ctx.type_context_reborrow(),
                receiver_ty_id,
                NormalizationMode::Assign,
                RelationMode::ASSIGN,
            );
            let receiver_ty_id = ctx.types.unwrap_value_type_id(receiver_ty_id);
            if self.type_is_solver_placeholder(receiver_ty_id, ctx.types) {
                continue;
            }

            // suppress cascades after primary receiver failure
            if self.type_blocks_cascading_diagnostic(receiver_ty_id, ctx.types) {
                continue;
            }

            // fail closed when discharge still depends on unsolved state
            if self.type_relation_requires_infer_convergence(
                ctx.type_view(),
                receiver_ty_id,
                receiver_ty_id,
            ) {
                if self.type_blocks_cascading_diagnostic(receiver_ty_id, ctx.types) {
                    continue;
                }

                return Err(AnalyzeError::Internal {
                    message: "unresolved missing member obligation after infer convergence"
                        .to_string(),
                });
            }

            // resolve member dispatch for the converged receiver
            let receiver_ty = ctx.types.get_type(receiver_ty_id).clone();
            let receiver_context = self.query_member_receiver_context_for_expression(
                &ctx.type_context_reborrow(),
                receiver_expression_id,
                Some(receiver_ty_id),
            );
            let member_resolution = self.resolve_member_symbol_for_receiver(
                &mut ctx.reborrow(),
                expression_id,
                receiver_expression_id,
                &receiver_ty,
                &receiver_context,
                &obligation.member_key,
            )?;
            let resolved_member_type_id = self.resolve_missing_member_obligation_value_type(
                &mut ctx.reborrow(),
                expression_id,
                receiver_expression_id,
                receiver_ty_id,
                &receiver_ty,
                &obligation.member_key,
            )?;
            if let Some(resolution) =
                self.resolve_missing_member_obligation_resolution(&member_resolution)
            {
                actions.push(MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolved_member_type_id,
                    resolution,
                });
                continue;
            }

            // keep structural member typing when symbol resolution stays unresolved
            if resolved_member_type_id.is_some() {
                actions.push(MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolved_member_type_id,
                    resolution: MissingMemberObligationResolution::Unresolved,
                });
                continue;
            }

            // resolve index signature value access path
            let mut index_visited = Vec::new();
            let index_type_id = self.resolve_index_signature_value_type_for_key(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                &receiver_ty,
                &obligation.member_key,
                &mut index_visited,
            );
            if index_type_id.is_some() {
                actions.push(MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolved_member_type_id: index_type_id,
                    resolution: MissingMemberObligationResolution::Unresolved,
                });
                continue;
            }

            // emit at most one diagnostic per expression id
            if !reported.insert(expression_id.id) {
                continue;
            }

            // report missing member diagnostics for unresolved lookups
            let allow_associated_contract_blocker = self
                .solve_projection_receiver_expression_for_missing_member_obligation(
                    &mut ctx.type_context_reborrow(),
                    receiver_expression_id,
                );
            let blocker = self.should_block_missing_member_diagnostic(
                ctx.type_view(),
                receiver_ty_id,
                allow_associated_contract_blocker,
            )?;
            if blocker.is_some() {
                continue;
            }

            actions.push(MissingMemberObligationAction::EmitMissingMemberDiagnostic {
                expression_id,
                receiver_ty_id,
                member_key: obligation.member_key,
            });
        }

        Ok(actions)
    }

    /// Resolve one deferred member expression value type from a converged receiver.
    fn resolve_missing_member_obligation_value_type(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        member_key: &StaticKey,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let receiver_context = self.query_member_receiver_context_for_expression(
            &ctx.type_context_reborrow(),
            receiver_expression_id,
            Some(receiver_ty_id),
        );

        let mut member_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            receiver_ty,
            member_key,
            receiver_context.lookup_mode,
            &mut member_visited,
        )?;

        Ok(member_ty_id)
    }

    /// Return true when one receiver should be treated as an associated projection in solve.
    fn solve_projection_receiver_expression_for_missing_member_obligation(
        &self,
        ctx: &mut TypeContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> bool {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, ctx.tree);
        if ctx
            .tree
            .get(receiver_id)
            .generic_arguments()
            .is_none_or(|arguments| arguments.is_empty())
        {
            return false;
        }

        // only treat direct type-space references as projection receivers
        let receiver_type_id = match ctx.tree.get(receiver_id) {
            Expression::Type { value, .. } => *value,
            _ => return false,
        };

        let symbol = self
            .resolve_direct_receiver_symbol_for_type_expression(&*ctx, receiver_type_id)
            .or_else(|| {
                let receiver_type_id = self
                    .resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        receiver_type_id,
                        true,
                        true,
                    )
                    .ok()?;
                self.query_type_like_receiver_symbol_for_type_id(receiver_type_id, ctx.types)
            });
        let Some(symbol) = symbol else {
            return false;
        };

        let symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), symbol)
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

    /// Apply missing-member obligation actions in solve.
    fn apply_missing_member_obligation_actions_in_solve(
        &self,
        ctx: &mut InferContext<'_>,
        actions: Vec<MissingMemberObligationAction>,
    ) -> AnalyzeResult<()> {
        for action in actions {
            match action {
                MissingMemberObligationAction::SetResolution {
                    expression_id,
                    receiver_ty_id,
                    resolved_member_type_id,
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
                    let expression_global = expression_id.into_global_any(ctx.module.id);
                    ctx.infer
                        .set_provisional_resolution_for_node(expression_global, resolution);
                    if let Some(resolved_member_type_id) = resolved_member_type_id {
                        ctx.infer
                            .set_inferred_type_for_node(expression_global, resolved_member_type_id);
                    }
                }
                MissingMemberObligationAction::EmitMissingMemberDiagnostic {
                    expression_id,
                    receiver_ty_id,
                    member_key,
                } => {
                    self.error(AnalyzeError::MissingMember {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        receiver_ty: receiver_ty_id.into_global(ctx.module.id),
                        member_key,
                    });
                    let error_type_id = ctx.types.insert_type_from(Type::Error, expression_id);
                    ctx.infer.set_inferred_type_for_node(
                        expression_id.into_global_any(ctx.module.id),
                        error_type_id,
                    );
                }
            }
        }

        Ok(())
    }
}
