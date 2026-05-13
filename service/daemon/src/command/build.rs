use std::collections::HashSet;

use destack_artifact::ArtifactKey;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use super::CommandResult;
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
    ) -> CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // resolve the target configuration for each module
        let target_overrides = self.common.target_overrides.as_ref();
        let mut module_targets = Vec::new();
        let mut target_ids = HashSet::new();
        for module_id in &modules {
            let target = self.resolve_target_for_module(revision, *module_id, target_overrides)?;
            target_ids.insert(target.id);
            module_targets.push((*module_id, target));
        }

        // collect build roots
        let mut artifact_keys = Vec::new();
        for (module_id, target) in &module_targets {
            artifact_keys.push(build_root_for_target(*module_id, target));
        }

        // provide the requested build roots
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let diagnostics = self
            .repository
            .diagnostics(revision, None)
            .map_err(|error| error.to_string())?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = module_targets
            .iter()
            .map(|(module_id, target)| self.target_profile_id(revision, *module_id, target.id))
            .collect::<Result<HashSet<_>, _>>()?
            .len();

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            target_ids.len(),
        ))
    }
}

/// Build the requested build root for one target.
fn build_root_for_target(module_id: ModuleId, target: &ResolvedTarget) -> ArtifactKey {
    ArtifactKey::module_output(module_id, target.id)
}
