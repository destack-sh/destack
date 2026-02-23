use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{
    AssociatedComptimeProjectionObligation, MissingMemberObligation, NodeTree, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Build one infer-table-missing internal error for commit stage paths.
    fn missing_commit_infer_table_error(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        context: &'static str,
    ) -> AnalyzeError {
        AnalyzeError::Internal {
            message: format!(
                "missing infer table for commit stage ({context}): module={module_id:?}, profile={profile:?}"
            ),
        }
    }

    /// Commit solved infer state for one module after solve convergence.
    pub(super) fn commit_module_solved_infer_state(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let options = self.analyze_context_options_for_module(module.id);

        // publish baseline solved state before projection replay
        self.publish_commit_baseline_state(module, profile)?;

        // replay deferred projection obligations from converged infer state
        self.commit_projection_obligations(module, profile)?;

        // discharge deferred instance commit obligations
        self.discharge_deferred_instance_commits(module, profile)?;

        // commit direct type writes that depend on solved type substitutions
        {
            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            self.commit_instance_inferred_types(module, profile, &tree, &symbols)?;
            self.commit_direct_binding_value_types(module, profile, &tree, &symbols)?;
        }

        // replay deferred missing-member obligations
        self.commit_missing_member_obligations(module, profile, &options)?;

        // replay deferred type relation obligations
        {
            let symbols = module.dir(profile).symbols.read();
            self.replay_commit_type_relation_obligations(module, profile, &symbols, &options)?;
        }

        // clear infer table only after successful commit replay
        self.clear_infer_table_for_module(module.id, profile);
        Ok(())
    }

    /// Publish baseline solved infer state to canonical tables.
    fn publish_commit_baseline_state(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let dir = module.dir(profile);
        let result = self
            .with_infer_table_for_module(module_id, profile, |infer| {
                let mut types = dir.types.write();
                self.commit_provisional_resolutions_and_instances(module, infer, &mut types)?;
                self.publish_inferred_expression_overlays(infer, &mut types);

                Ok::<(), AnalyzeError>(())
            })
            .ok_or_else(|| {
                self.missing_commit_infer_table_error(module_id, profile, "publish baseline")
            })?;
        result?;

        Ok(())
    }

    /// Discharge deferred instance commit obligations to canonical tables.
    fn discharge_deferred_instance_commits(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let dir = module.dir(profile);
        let result = self
            .with_infer_table_for_module_mut(module_id, profile, |infer| {
                let mut types = dir.types.write();
                self.discharge_instance_commit_obligations(infer, &mut types)?;

                Ok::<(), AnalyzeError>(())
            })
            .ok_or_else(|| {
                self.missing_commit_infer_table_error(module_id, profile, "discharge instance")
            })?;
        result?;

        Ok(())
    }

    /// Take and sort projection obligations for deterministic commit replay.
    fn take_projection_obligations(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<Vec<AssociatedComptimeProjectionObligation>> {
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            let mut obligations = infer.take_associated_comptime_projection_obligations();
            obligations.sort_by_key(|obligation| obligation.expression_id.id);
            obligations
        })
        .ok_or_else(|| self.missing_commit_infer_table_error(module_id, profile, "take projection"))
    }

    /// Restore projection obligations when replay yields on a dependency.
    fn restore_projection_obligations(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        obligations: Vec<AssociatedComptimeProjectionObligation>,
    ) -> AnalyzeResult<()> {
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            for obligation in obligations {
                infer.push_associated_comptime_projection_obligation(obligation);
            }
        })
        .ok_or_else(|| {
            self.missing_commit_infer_table_error(module_id, profile, "restore projection")
        })?;

        Ok(())
    }

    /// Replay projection obligations against converged infer state.
    fn commit_projection_obligations(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let obligations = self.take_projection_obligations(module_id, profile)?;
        if obligations.is_empty() {
            return Ok(());
        }

        let dir = module.dir(profile);
        let mut projection_snapshot = {
            let types = dir.types.read();
            types.clone()
        };

        let projection_actions = match self.collect_projection_commit_actions(
            module,
            profile,
            &obligations,
            &mut projection_snapshot,
        ) {
            Ok(actions) => actions,
            Err(AnalyzeError::Yield { dependency }) => {
                self.restore_projection_obligations(module_id, profile, obligations)?;
                return Err(AnalyzeError::Yield { dependency });
            }
            Err(error) => return Err(error),
        };

        let mut types = dir.types.write();
        self.apply_projection_commit_actions(
            module,
            profile,
            projection_actions,
            &projection_snapshot,
            &mut types,
        )?;
        Ok(())
    }

    /// Collect and apply instance-inferred type commits.
    fn commit_instance_inferred_types(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let dir = module.dir(profile);
        let mut snapshot = {
            let types = dir.types.read();
            types.clone()
        };

        let actions = self
            .with_infer_table_for_module(module_id, profile, |infer| {
                self.collect_instance_inferred_type_commit_actions(
                    module,
                    profile,
                    tree,
                    symbols,
                    infer,
                    &mut snapshot,
                )
            })
            .ok_or_else(|| {
                self.missing_commit_infer_table_error(module_id, profile, "collect instance")
            })??;
        if actions.is_empty() {
            return Ok(());
        }

        let mut types = dir.types.write();
        self.apply_instance_inferred_type_commit_actions(actions, &snapshot, &mut types)?;
        Ok(())
    }

    /// Collect and apply direct-binding value-type commits.
    fn commit_direct_binding_value_types(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let dir = module.dir(profile);
        let mut snapshot = {
            let types = dir.types.read();
            types.clone()
        };

        let actions = self
            .with_infer_table_for_module(module_id, profile, |infer| {
                self.collect_direct_binding_value_type_commit_actions(
                    module,
                    profile,
                    tree,
                    symbols,
                    &mut snapshot,
                    infer,
                )
            })
            .ok_or_else(|| {
                self.missing_commit_infer_table_error(module_id, profile, "collect binding")
            })?;
        if actions.is_empty() {
            return Ok(());
        }

        let mut types = dir.types.write();
        self.apply_direct_binding_value_type_commit_actions(actions, &snapshot, &mut types)?;
        Ok(())
    }

    /// Take and sort missing-member obligations for deterministic replay.
    fn take_missing_member_obligations(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<Vec<MissingMemberObligation>> {
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            let mut obligations = infer.take_missing_member_obligations();
            obligations.sort_by_key(|obligation| {
                (obligation.expression_id, obligation.receiver_expression_id)
            });
            obligations
        })
        .ok_or_else(|| {
            self.missing_commit_infer_table_error(module_id, profile, "take missing member")
        })
    }

    /// Restore missing-member obligations when replay yields on a dependency.
    fn restore_missing_member_obligations(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        obligations: Vec<MissingMemberObligation>,
    ) -> AnalyzeResult<()> {
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            for obligation in obligations {
                infer.push_missing_member_obligation(obligation);
            }
        })
        .ok_or_else(|| {
            self.missing_commit_infer_table_error(module_id, profile, "restore missing member")
        })?;

        Ok(())
    }

    /// Replay missing-member obligations against converged infer state.
    fn commit_missing_member_obligations(
        &self,
        module: &Module,
        profile: ProfileId,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let obligations = self.take_missing_member_obligations(module_id, profile)?;
        if obligations.is_empty() {
            return Ok(());
        }

        let dir = module.dir(profile);
        let mut snapshot = {
            let types = dir.types.read();
            types.clone()
        };

        let actions = match self
            .with_infer_table_for_module(module_id, profile, |infer| {
                self.collect_missing_member_commit_actions(
                    module,
                    profile,
                    &obligations,
                    infer,
                    &mut snapshot,
                    options,
                )
            })
            .ok_or_else(|| {
                self.missing_commit_infer_table_error(module_id, profile, "collect missing member")
            })? {
            Ok(actions) => actions,
            Err(AnalyzeError::Yield { dependency }) => {
                self.restore_missing_member_obligations(module_id, profile, obligations)?;
                return Err(AnalyzeError::Yield { dependency });
            }
            Err(error) => return Err(error),
        };

        let mut types = dir.types.write();
        self.apply_missing_member_commit_actions(module, profile, actions, &snapshot, &mut types)?;
        Ok(())
    }

    /// Take and replay type relation obligations after convergence.
    fn replay_commit_type_relation_obligations(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let obligations = self
            .with_infer_table_for_module_mut(module_id, profile, |infer| {
                let mut obligations = infer.take_type_relation_obligations();
                obligations.sort_by_key(|obligation| obligation.source_node_id);
                obligations
            })
            .ok_or_else(|| {
                self.missing_commit_infer_table_error(module_id, profile, "take relation")
            })?;
        if obligations.is_empty() {
            return Ok(());
        }

        let dir = module.dir(profile);
        let mut snapshot = {
            let types = dir.types.read();
            types.clone()
        };
        self.replay_type_relation_obligations_from_records(
            module,
            profile,
            symbols,
            &mut snapshot,
            &obligations,
            options,
        )?;
        Ok(())
    }
}
