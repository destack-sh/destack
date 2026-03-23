use std::collections::HashSet;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::ModuleId;
use destack_workspace::{Program, TargetId};
use serde::{Deserialize, Serialize};

use super::context::CommandContext;
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

        // resolve the target configuration for each module
        let target_overrides = self.common.target_overrides.as_ref();
        let mut module_targets = Vec::new();
        let mut target_ids = HashSet::new();
        for module_id in &modules {
            let target_id = self.resolve_target_for_module(*module_id, target_overrides)?;
            target_ids.insert(target_id.clone());
            module_targets.push((*module_id, target_id));
        }

        // enqueue build tasks
        self.reset_diagnostics();
        enqueue_build_tasks(&self.program, &self.compiler, &module_targets);

        // compile and collect diagnostics
        self.compiler.compile();
        let raw_diagnostics = self.collect_raw_diagnostics();
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let stats = self
            .compiler
            .stats
            .snapshot_with_program(self.program.modules.len(), Some(&self.program));

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            self.program.profiles.len(),
            target_ids.len(),
            Some(stats),
        ))
    }
}

/// Enqueue build tasks for the provided modules.
fn enqueue_build_tasks(
    program: &Arc<Program>,
    compiler: &Arc<Compiler>,
    module_targets: &[(ModuleId, TargetId)],
) {
    for (module_id, target_id) in module_targets {
        let _profile = program.profile_id_for_target_or_default(*module_id, target_id);
        compiler.enqueue(ArtifactKey::module_artifact(*module_id, target_id.clone()));
    }
}
