use crate::analyze::common::StaticSubstitutionEnvironment;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DynamicResolutionCandidateSlotId, GlobalNodeIdAny, InferTable, InstanceCommitObligationId,
    LocalInstanceId, LocalResolutionId, Resolution, TypeTable,
};
use std::collections::HashMap;

impl Compiler {
    /// Return one committed instance id for one obligation attachment.
    fn committed_instance_for_obligation(
        &self,
        instance_by_obligation: &HashMap<InstanceCommitObligationId, LocalInstanceId>,
        obligation_id: InstanceCommitObligationId,
    ) -> AnalyzeResult<LocalInstanceId> {
        instance_by_obligation
            .get(&obligation_id)
            .copied()
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
        let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
            return Ok(());
        };
        let Resolution::Static { candidate, .. } = types.get_resolution_mut(resolution_id) else {
            return Ok(());
        };

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
        resolution_id: LocalResolutionId,
        candidate_slot: DynamicResolutionCandidateSlotId,
        instance_id: LocalInstanceId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let resolution = types.get_resolution_mut(resolution_id);
        match resolution {
            Resolution::Dynamic { candidates, .. } | Resolution::Unresolved { candidates, .. } => {
                let candidate_index = candidate_slot.0 as usize;
                let Some(candidate) = candidates.get_mut(candidate_index) else {
                    return Err(AnalyzeError::Internal {
                        message: "missing dynamic resolution candidate slot".to_string(),
                    });
                };
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

    /// Discharge instance-commit obligations after infer convergence.
    pub(crate) fn discharge_instance_commit_obligations(
        &self,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let obligations = infer
            .iter_instance_commit_obligations()
            .map(|(obligation_id, obligation)| (obligation_id, obligation.clone()))
            .collect::<Vec<_>>();

        let mut instance_by_obligation =
            HashMap::<InstanceCommitObligationId, LocalInstanceId>::new();
        for (obligation_id, obligation) in obligations {
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
            instance_by_obligation.insert(obligation_id, instance_id);
        }

        let node_attachments = infer
            .iter_instance_commit_obligation_nodes()
            .collect::<Vec<_>>();
        for (node_id, obligation_id) in node_attachments {
            let instance_id =
                self.committed_instance_for_obligation(&instance_by_obligation, obligation_id)?;
            types.set_instance_for_node(node_id, instance_id);
            self.attach_instance_to_static_resolution_for_node(node_id, instance_id, types)?;
        }

        let resolution_attachments = infer
            .iter_instance_commit_obligation_resolution_candidates()
            .collect::<Vec<_>>();
        for attachment in resolution_attachments {
            let instance_id = self.committed_instance_for_obligation(
                &instance_by_obligation,
                attachment.obligation_id,
            )?;
            self.attach_instance_to_resolution_candidate(
                attachment.resolution_id,
                attachment.candidate_slot,
                instance_id,
                types,
            )?;
        }

        Ok(())
    }
}
