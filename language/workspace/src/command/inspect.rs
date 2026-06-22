use destack_serde::Schema;
use std::collections::BTreeSet;
use std::path::PathBuf;

use destack_artifact::{ArtifactKey, ArtifactRecord};
use destack_core::StringPool;
use destack_repository::Revision;
use destack_source::{ModuleId, ProfileId, TargetId};
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::{CommandContext, SelectedTarget};
use super::outcome::CommandOutcome;
/// Options for the inspect command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct InspectOptions {
    /// The artifact view to inspect.
    pub view: InspectView,
}

/// Inspectable compiler artifact view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum InspectView {
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
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct InspectPayload {
    /// Inspected artifact records.
    pub artifacts: Vec<ArtifactRecord>,
    /// String pool needed to render inspected artifacts.
    pub strings: StringPool,
}

/// Request to inspect compiler artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct InspectInput {
    /// Revision selected for this inspect request.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether destack.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional Destack manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// The artifact view to inspect.
    pub view: InspectView,
}

impl_command_input_options!(InspectInput {
    view: InspectView::Diagnostics,
});

impl CommandContext<'_> {
    /// Execute an inspect command.
    pub(crate) fn run_inspect_command(
        &mut self,
        options: &InspectOptions,
    ) -> CommandResult<CommandOutcome<InspectPayload>> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // diagnostics are a check shaped inspect view
        if options.view == InspectView::Diagnostics {
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
        let payload = InspectPayload {
            artifacts,
            strings: self.repository.string_pool().as_ref().clone(),
        };
        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            target_ids.len(),
        )
        .with_data(payload))
    }

    /// Execute diagnostics inspect.
    fn run_diagnostics_inspect(
        &mut self,
        revision: Revision,
        modules: &[ModuleId],
    ) -> CommandResult<CommandOutcome<InspectPayload>> {
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

        let payload = InspectPayload {
            artifacts: Vec::new(),
            strings: self.repository.string_pool().as_ref().clone(),
        };

        Ok(
            CommandOutcome::new(diagnostics, exit_code, modules.len(), profile_count, 0)
                .with_data(payload),
        )
    }

    /// Return one artifact record for a revision-scoped key.
    fn artifact_record(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> CommandResult<ArtifactRecord> {
        let version = self
            .repository
            .artifact_version(revision, &key)
            .map_err(|error| format!("failed to read artifact version: {error}"))?
            .ok_or_else(|| format!("missing inspected artifact: {key:?}"))?;
        let record = self
            .repository
            .artifact_table()
            .record(&version, self.repository.string_pool())
            .map_err(|error| format!("failed to serialize artifact record: {error}"))?
            .ok_or_else(|| format!("missing inspected artifact payload: {key:?}"))?;

        Ok(record)
    }

    /// Count unique profiles used by inspect targets.
    fn inspect_profile_count(
        &self,
        revision: Revision,
        module_targets: &[(ModuleId, SelectedTarget)],
    ) -> CommandResult<usize> {
        let mut profiles = BTreeSet::new();

        for (module, target) in module_targets {
            profiles.insert(self.target_profile_id(revision, *module, target.id)?);
        }

        Ok(profiles.len())
    }
}

impl InspectView {
    /// Build the artifact key for this inspect view.
    fn artifact_key(self, module: ModuleId, profile: ProfileId, target: TargetId) -> ArtifactKey {
        match self {
            Self::Diagnostics => ArtifactKey::dir_checked(module, profile),
            Self::MirLowered => ArtifactKey::mir_lowered(module, profile, target),
            Self::MirVerified => ArtifactKey::mir_verified(module, profile, target),
            Self::MirOptimized => ArtifactKey::mir_optimized(module, profile, target),
        }
    }
}
