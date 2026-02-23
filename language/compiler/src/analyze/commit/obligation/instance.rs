use crate::analyze::StaticSubstitutionEnvironment;
use crate::analyze::common::TypeRewriteCache;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DynamicResolutionCandidateSlotId, GlobalNodeIdAny, InferTable, InstanceCommitObligationId,
    LocalInstanceId, LocalTypeId, NodeTree, Resolution, StaticArgument, StaticExpression,
    SymbolTable, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashMap;

/// One inferred-type writeback action produced by instance-substitution collection.
#[derive(Debug, Clone, Copy)]
pub(in crate::analyze::commit) struct InstanceInferredTypeCommitAction {
    /// The node that should receive an updated inferred type.
    pub node_id: GlobalNodeIdAny,
    /// The mapped inferred type id in the collection snapshot.
    pub mapped_type_id: LocalTypeId,
}

impl Compiler {
    /// Return one committed instance id for one obligation attachment.
    fn committed_instance_for_obligation(
        &self,
        instance_by_obligation: &[Option<LocalInstanceId>],
        obligation_id: InstanceCommitObligationId,
    ) -> AnalyzeResult<LocalInstanceId> {
        // read one committed instance id from the obligation index
        instance_by_obligation
            .get(obligation_id.0 as usize)
            .copied()
            .flatten()
            .ok_or(AnalyzeError::Internal {
                message: "missing committed instance".to_string(),
            })
    }

    /// Attach one committed instance id to a static resolution candidate when missing.
    fn attach_instance_to_static_resolution_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        instance_id: LocalInstanceId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // skip when no resolution is attached to this node
        let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
            return Ok(());
        };

        // only static resolutions can be attached through this path
        let Resolution::Static { candidate, .. } = types.get_resolution_mut(resolution_id) else {
            return Ok(());
        };

        // reject conflicting pre existing instance ids
        if let Some(existing) = candidate.instance {
            if existing != instance_id {
                return Err(AnalyzeError::Internal {
                    message: "conflicting static resolution instance".to_string(),
                });
            }
            return Ok(());
        }

        candidate.instance = Some(instance_id);
        Ok(())
    }

    /// Attach one committed instance id to one dynamic resolution candidate when missing.
    fn attach_instance_to_resolution_candidate(
        &self,
        node_id: GlobalNodeIdAny,
        candidate_slot: DynamicResolutionCandidateSlotId,
        instance_id: LocalInstanceId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
            return Ok(());
        };

        // attach only to dynamic or unresolved candidate lists
        let resolution = types.get_resolution_mut(resolution_id);
        match resolution {
            Resolution::Dynamic { candidates, .. } | Resolution::Unresolved { candidates, .. } => {
                // resolve the candidate slot from the attachment record
                let candidate_index = candidate_slot.0 as usize;
                let Some(candidate) = candidates.get_mut(candidate_index) else {
                    return Err(AnalyzeError::Internal {
                        message: "missing dynamic resolution candidate slot".to_string(),
                    });
                };
                // reject conflicting pre existing instance ids
                if let Some(existing) = candidate.instance {
                    if existing != instance_id {
                        return Err(AnalyzeError::Internal {
                            message: "conflicting dynamic resolution candidate instance"
                                .to_string(),
                        });
                    }
                    return Ok(());
                }

                candidate.instance = Some(instance_id);
                Ok(())
            }
            Resolution::Static { .. } | Resolution::Builtin { .. } => Err(AnalyzeError::Internal {
                message: "dynamic attachment targeted non-dynamic resolution".to_string(),
            }),
        }
    }

    /// Discharge instance commit obligations after infer convergence.
    pub(in crate::analyze::commit) fn discharge_instance_commit_obligations(
        &self,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // commit one instance id for each obligation id
        let obligation_count = infer.instance_commit_obligation_count();
        let mut instance_by_obligation = vec![None; obligation_count];
        for (obligation_id, obligation) in infer.iter_instance_commit_obligations() {
            let environment = StaticSubstitutionEnvironment::from_parameter_symbols(
                obligation.static_arguments.clone(),
                obligation.static_parameter_symbols.clone(),
                obligation.inherited_static_argument_count,
            )
            .ok_or_else(|| AnalyzeError::Internal {
                message: "invalid obligation environment".to_string(),
            })?;
            let instance_id = self.commit_instance_for_symbol_environment(
                obligation.symbol_id,
                environment,
                types,
            )?;
            instance_by_obligation[obligation_id.0 as usize] = Some(instance_id);
        }

        // attach committed instance ids to node bindings in deterministic node order
        let mut node_attachments = infer
            .iter_instance_commit_obligation_nodes()
            .collect::<Vec<_>>();
        node_attachments.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, obligation_id) in node_attachments {
            let instance_id =
                self.committed_instance_for_obligation(&instance_by_obligation, obligation_id)?;
            types.set_instance_for_node(node_id, instance_id);
            self.attach_instance_to_static_resolution_for_node(node_id, instance_id, types)?;
        }

        // attach committed instance ids to dynamic resolution candidates in deterministic order
        let mut resolution_attachments = infer
            .iter_instance_commit_obligation_resolution_candidates()
            .collect::<Vec<_>>();
        resolution_attachments.sort_by_key(|attachment| {
            (
                attachment.node_id,
                attachment.candidate_slot.0,
                attachment.obligation_id.0,
            )
        });
        for attachment in resolution_attachments {
            let instance_id = self.committed_instance_for_obligation(
                &instance_by_obligation,
                attachment.obligation_id,
            )?;
            self.attach_instance_to_resolution_candidate(
                attachment.node_id,
                attachment.candidate_slot,
                instance_id,
                types,
            )?;
        }

        Ok(())
    }

    /// Collect inferred-type writeback actions that depend on resolved instance substitutions.
    pub(in crate::analyze::commit) fn collect_instance_inferred_type_commit_actions(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Vec<InstanceInferredTypeCommitAction>> {
        // apply substitutions to inferred types using explicit infer obligations
        let mut node_attachments = infer
            .iter_instance_commit_obligation_nodes()
            .collect::<Vec<_>>();
        node_attachments.sort_by_key(|(node_id, _)| *node_id);
        let mut actions = Vec::new();
        for (node_id, obligation_id) in node_attachments {
            // skip attachments that do not belong to this module
            if node_id.module_id != module.id {
                continue;
            }

            // skip nodes without inferred type commits
            let Some(inferred_type_id) = types.get_inferred_type_id(node_id) else {
                continue;
            };
            let Some(obligation) = infer.instance_commit_obligation(obligation_id) else {
                return Err(AnalyzeError::Internal {
                    message: "missing instance commit obligation for inferred node".to_string(),
                });
            };

            // build type substitutions from the committed obligation environment
            let mut substitutions = HashMap::new();
            for (parameter_symbol, argument) in obligation
                .static_parameter_symbols
                .iter()
                .zip(obligation.static_arguments.iter())
            {
                let StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } = argument
                else {
                    continue;
                };
                substitutions.insert(*parameter_symbol, types.unwrap_value_type_id(*ty));
            }
            if substitutions.is_empty() {
                continue;
            }

            // instantiate inferred type with committed substitutions
            let mut materialize_cache = TypeRewriteCache::new();
            let mut substitution_cache = HashMap::new();
            let mapped_type_id = self.instantiate_type_with_substitutions(
                module,
                profile,
                node_id.local_id,
                None,
                inferred_type_id,
                &substitutions,
                tree,
                symbols,
                types,
                &mut materialize_cache,
                &mut substitution_cache,
            );

            // commit mapped type ids when instantiation changed the result
            if mapped_type_id != inferred_type_id {
                actions.push(InstanceInferredTypeCommitAction {
                    node_id,
                    mapped_type_id,
                });
            }
        }

        Ok(actions)
    }

    /// Apply inferred-type writeback actions for instance-substitution commits.
    pub(in crate::analyze::commit) fn apply_instance_inferred_type_commit_actions(
        &self,
        actions: Vec<InstanceInferredTypeCommitAction>,
        snapshot_types: &TypeTable,
        committed_types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let committed_type_floor = committed_types.type_count();
        let mut is_synchronized = false;
        for action in actions {
            let committed_type_id = self.committed_type_id_for_snapshot_type(
                action.mapped_type_id,
                snapshot_types,
                committed_types,
                committed_type_floor,
                &mut is_synchronized,
            )?;
            committed_types.set_inferred_type(action.node_id, committed_type_id);
        }

        Ok(())
    }
}
