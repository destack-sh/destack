use std::collections::BTreeSet;

use destack_artifact::{ArtifactKey, ArtifactRecord};
use destack_source::{ModuleId, TargetId};
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::{CommandContext, SelectedTarget};
use super::dispatch::CommandOutcome;

/// Options for the inspect command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandInspectOptions {
    /// The artifact view to inspect.
    pub view: CommandInspectView,
}

/// Inspectable compiler artifact view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandInspectView {
    /// Diagnostics produced by checking the input.
    Diagnostics,
    /// Lowered MIR before verification and optimization.
    MirLowered,
    /// Verified MIR after required semantic rewrites.
    MirVerified,
    /// Optimized MIR after optimization patches.
    MirOptimized,
}

/// Payload for inspect command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInspectPayload {
    /// Inspected artifact records.
    pub artifacts: Vec<ArtifactRecord>,
}

impl CommandContext<'_> {
    /// Execute an inspect command.
    pub(super) fn run_inspect_command(
        &mut self,
        options: &CommandInspectOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // diagnostics are a check shaped inspect view
        if options.view == CommandInspectView::Diagnostics {
            return self.run_diagnostics_inspect(revision, &modules);
        }

        // resolve target configuration for each module
        let target_overrides = self.common.target_overrides.as_ref();
        let mut module_targets = Vec::new();
        let mut target_ids = BTreeSet::new();
        for module in &modules {
            let target = self.resolve_target_for_module(revision, *module, target_overrides)?;
            target_ids.insert(target.id);
            module_targets.push((*module, target));
        }

        // provide requested artifact roots
        let mut roots = Vec::new();
        for (module, target) in &module_targets {
            let profile = self.target_profile_id(revision, *module, target.id)?;
            roots.push(options.view.artifact_key(*module, profile, target.id));
        }
        self.session
            .provide(revision, &roots)
            .map_err(|error| error.to_string())?;

        // collect requested artifact records
        let mut artifacts = Vec::new();
        for key in roots {
            let record = self.artifact_record(revision, key)?;
            artifacts.push(record);
        }

        // collect command diagnostics and payload
        let diagnostics = self
            .repository
            .diagnostics(revision, None)
            .map_err(|error| error.to_string())?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = self.inspect_profile_count(revision, &module_targets)?;
        let payload = CommandInspectPayload { artifacts };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid inspect payload: {error}"))?;

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            target_ids.len(),
        )
        .with_data(data))
    }

    /// Execute diagnostics inspect.
    fn run_diagnostics_inspect(
        &mut self,
        revision: destack_workspace::Revision,
        modules: &[ModuleId],
    ) -> CommandResult<CommandOutcome> {
        // provide checked roots
        let mut roots = Vec::new();
        for module in modules {
            let profile = self.selected_profile_id(revision, *module)?;
            roots.push(ArtifactKey::dir_checked(*module, profile));
        }
        self.session
            .provide(revision, &roots)
            .map_err(|error| error.to_string())?;

        // collect diagnostics
        let diagnostics = self
            .repository
            .diagnostics(revision, None)
            .map_err(|error| error.to_string())?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = self.selected_profile_count(revision, modules)?;

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
        ))
    }

    /// Return one artifact record for a revision-scoped key.
    fn artifact_record(
        &self,
        revision: destack_workspace::Revision,
        key: ArtifactKey,
    ) -> CommandResult<ArtifactRecord> {
        let version = self
            .repository
            .artifact_version(revision, &key)
            .map_err(|error| format!("failed to read artifact version: {error}"))?
            .ok_or_else(|| format!("missing inspected artifact: {key:?}"))?;
        let record = self
            .repository
            .artifact_store()
            .record(&version, self.repository.string_pool())
            .map_err(|error| format!("failed to serialize artifact record: {error}"))?
            .ok_or_else(|| format!("missing inspected artifact payload: {key:?}"))?;

        Ok(record)
    }

    /// Count unique profiles used by inspect targets.
    fn inspect_profile_count(
        &self,
        revision: destack_workspace::Revision,
        module_targets: &[(ModuleId, SelectedTarget)],
    ) -> CommandResult<usize> {
        let mut profiles = BTreeSet::new();

        for (module, target) in module_targets {
            profiles.insert(self.target_profile_id(revision, *module, target.id)?);
        }

        Ok(profiles.len())
    }
}

impl CommandInspectView {
    /// Build the artifact key for this inspect view.
    fn artifact_key(
        self,
        module: ModuleId,
        profile: destack_source::ProfileId,
        target: TargetId,
    ) -> ArtifactKey {
        match self {
            Self::Diagnostics => ArtifactKey::dir_checked(module, profile),
            Self::MirLowered => ArtifactKey::mir_lowered(module, profile, target),
            Self::MirVerified => ArtifactKey::mir_verified(module, profile, target),
            Self::MirOptimized => ArtifactKey::mir_optimized(module, profile, target),
        }
    }
}
