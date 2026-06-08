use std::path::{Path, PathBuf};

use destack_repository::{Repository, Revision, parse_jsonc_text};
use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the manifest command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandManifestOptions {
    /// Optional manifest path override.
    pub path: Option<PathBuf>,
    /// Whether to include full manifest output.
    pub full: bool,
}

/// Payload for manifest command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandManifestPayload {
    /// Path to the manifest file.
    pub path: String,
    /// Default target name.
    pub default_target: Option<String>,
    /// Configured target names.
    pub targets: Vec<String>,
    /// Raw manifest json.
    pub manifest: serde_json::Value,
    /// Whether full output was requested.
    pub full: bool,
}

impl CommandContext<'_> {
    /// Execute a manifest command.
    pub(super) fn run_manifest_command(
        &mut self,
        options: &CommandManifestOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve manifest path overrides
        let manifest_override = options
            .path
            .as_deref()
            .or(self.common.manifest_path.as_deref());

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
        let payload = CommandManifestPayload {
            path: manifest_path.display().to_string(),
            default_target,
            targets: target_names,
            manifest: manifest_json,
            full: options.full,
        };

        // serialize command payload
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid manifest payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
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
