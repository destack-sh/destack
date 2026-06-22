use destack_serde::Schema;
use std::path::{Path, PathBuf};

use destack_repository::{Repository, Revision, parse_jsonc_text};
use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the manifest command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema, Default)]
pub struct ManifestOptions {
    /// Optional manifest path override.
    pub path: Option<PathBuf>,
    /// Whether to include full manifest output.
    pub full: bool,
}

/// Payload for manifest command output.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ManifestPayload {
    /// Path to the manifest file.
    pub path: String,
    /// Default target name.
    pub default_target: Option<String>,
    /// Configured target names.
    pub targets: Vec<String>,
    /// Raw manifest JSON.
    pub manifest: serde_json::Value,
    /// Whether full output was requested.
    pub full: bool,
}

/// Request to return resolved manifest information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ManifestInput {
    /// Revision selected for this manifest request.
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
    /// Optional manifest path override.
    pub path: Option<PathBuf>,
    /// Whether to include full manifest output.
    pub full: bool,
}

impl_command_input_options!(ManifestInput {
    path: None,
    full: false,
});

impl CommandContext<'_> {
    /// Execute a manifest command.
    pub(crate) fn run_manifest_command(
        &mut self,
        options: &ManifestOptions,
    ) -> CommandResult<CommandOutcome<ManifestPayload>> {
        // resolve manifest path overrides
        let manifest_override = options.path.as_deref().or(self.common.manifest.as_deref());

        // resolve manifest path
        let manifest_path = self.resolve_destack_config_path(manifest_override)?;

        // load manifest fields
        let manifest = self.load_destack_config(&manifest_path)?;

        // parse raw manifest json
        let revision = self.revision()?;
        let manifest_json = read_manifest_json(&self.repository, revision, &manifest_path)?;

        // collect target metadata
        let target_names: Vec<String> = manifest.targets.keys().cloned().collect();
        let default_target = manifest.default_target.clone();

        // build the manifest payload
        let payload = ManifestPayload {
            path: manifest_path.display().to_string(),
            default_target,
            targets: target_names,
            manifest: manifest_json,
            full: options.full,
        };

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}

/// Read and parse a destack.json file into JSON.
fn read_manifest_json(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> CommandResult<serde_json::Value> {
    let file_id = repository.file_id(path);
    let file = repository
        .file(revision, file_id)
        .map_err(|error| format!("failed to load {}: {error}", path.display()))?
        .ok_or_else(|| format!("failed to load {}", path.display()))?;
    let content = file.text();

    parse_jsonc_text(content).map_err(|error| format!("invalid destack.json: {error}").into())
}
