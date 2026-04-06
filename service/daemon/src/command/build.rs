use std::collections::HashSet;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::{DiagnosticCollection, ModuleId};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use super::context::{CommandContext, ResolvedTarget};
use super::dispatch::CommandOutcome;

/// Options for the build command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandBuildOptions {}

impl CommandContext<'_> {
    /// Execute a build command.
    pub(super) fn run_build_command(
        &mut self,
        _options: &CommandBuildOptions,
    ) -> super::CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // resolve the target configuration for each module
        let target_overrides = self.common.target_overrides.as_ref();
        let mut module_targets = Vec::new();
        let mut target_ids = HashSet::new();
        for module_id in &modules {
            let target = self.resolve_target_for_module(*module_id, target_overrides)?;
            target_ids.insert(target.id);
            module_targets.push((*module_id, target));
        }

        // enqueue build tasks
        enqueue_build_tasks(&self.repository, &self.compiler, revision, &module_targets)?;

        // compile and collect diagnostics
        self.compiler.compile();
        let raw_diagnostics =
            collect_module_target_diagnostics(&self.repository, revision, &module_targets)?;
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let module_count = self.module_count(revision)?;
        let profile_count = module_targets
            .iter()
            .map(|(module_id, target)| {
                self.repository
                    .profile_id_for_target_or_default(revision, *module_id, &target.id)
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<HashSet<_>, _>>()?
            .len();
        let stats = self
            .compiler
            .stats
            .snapshot_with_repository(module_count, Some(&self.repository));

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            target_ids.len(),
            Some(stats),
        ))
    }
}

/// Collect diagnostics across the current target artifact families for the requested modules.
fn collect_module_target_diagnostics(
    repository: &Arc<Repository>,
    revision: Revision,
    module_targets: &[(ModuleId, ResolvedTarget)],
) -> super::CommandResult<DiagnosticCollection> {
    let mut diagnostics = DiagnosticCollection::new();

    // current target families
    for (module_id, target) in module_targets {
        let profile_id = repository
            .profile_id_for_target_or_default(revision, *module_id, &target.id)
            .map_err(|error| error.to_string())?;
        diagnostics.merge_from(
            &repository
                .module_target_artifact_diagnostics(revision, *module_id, profile_id, target.id),
        );
    }

    Ok(diagnostics)
}

/// Enqueue build tasks for the provided modules.
fn enqueue_build_tasks(
    repository: &Arc<Repository>,
    compiler: &Arc<Compiler>,
    revision: Revision,
    module_targets: &[(ModuleId, ResolvedTarget)],
) -> super::CommandResult<()> {
    for (module_id, target) in module_targets {
        let _profile = repository
            .profile_id_for_target_or_default(revision, *module_id, &target.id)
            .map_err(|error| error.to_string())?;
        compiler.enqueue(revision, ArtifactKey::module_output(*module_id, target.id));
    }

    Ok(())
}
