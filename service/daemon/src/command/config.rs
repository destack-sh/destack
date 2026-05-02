use std::path::Path;

use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the config command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandConfigOptions {
    /// Optional config path override.
    pub path: Option<std::path::PathBuf>,
    /// Whether to include full config output.
    pub full: bool,
}

/// Payload for config command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfigPayload {
    /// Path to the config file.
    pub path: String,
    /// Default target name.
    pub default_target: Option<String>,
    /// Configured target names.
    pub targets: Vec<String>,
    /// Raw config json.
    pub config: serde_json::Value,
    /// Whether full output was requested.
    pub full: bool,
}

impl CommandContext<'_> {
    /// Execute a config command.
    pub(super) fn run_config_command(
        &mut self,
        options: &CommandConfigOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve config path overrides
        let config_override = options
            .path
            .as_deref()
            .or(self.common.config_path.as_deref());

        // resolve config path
        let config_path = self.resolve_destack_config_path(config_override)?;

        // load config for summary fields
        let declaration = self.load_destack_declaration(&config_path)?;
        let package_options = declaration.package_options();

        // parse raw config json
        let resolver = self.resolver();
        let config_json = read_config_json(&resolver, &config_path)?;

        // collect target metadata
        let target_names: Vec<String> = package_options.targets.keys().cloned().collect();
        let default_target = package_options.default_target.clone();

        let payload = CommandConfigPayload {
            path: config_path.display().to_string(),
            default_target,
            targets: target_names,
            config: config_json,
            full: options.full,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid config payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
    }
}

/// Read and parse a destack.json file into JSON.
fn read_config_json(
    resolver: &destack_resolver::Resolver,
    path: &Path,
) -> CommandResult<serde_json::Value> {
    let repository = resolver.repository();
    let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
    let revision = repository
        .current(&reference)
        .map_err(|error| error.to_string())?;
    let file_id = repository.file_id(path);
    let file = repository
        .file(revision, file_id)
        .map_err(|error| format!("failed to load {}: {error}", path.display()))?
        .ok_or_else(|| format!("failed to load {}", path.display()))?;
    let content = file.text();

    Ok(serde_json::from_str(content).map_err(|error| format!("invalid destack.json: {error}"))?)
}
