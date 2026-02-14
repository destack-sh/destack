use std::path::Path;

use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

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
    ) -> super::CommandResult<CommandOutcome> {
        // resolve config path overrides
        let config_override = options
            .path
            .as_deref()
            .or(self.common.config_path.as_deref());

        // resolve dsconfig path
        let dsconfig_path = self.resolve_dsconfig_path(config_override)?;

        // load dsconfig for summary fields
        let dsconfig = self.load_dsconfig(&dsconfig_path)?;

        // parse raw config json
        let resolver = self.resolver();
        let config_json = read_config_json(&resolver, &dsconfig_path)?;

        // collect target metadata
        let target_names: Vec<String> = dsconfig.options.targets.keys().cloned().collect();
        let default_target = dsconfig.options.default_target.clone();

        let payload = CommandConfigPayload {
            path: dsconfig_path.display().to_string(),
            default_target,
            targets: target_names,
            config: config_json,
            full: options.full,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid config payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0, None).with_data(data))
    }
}

/// Read and parse a dsconfig.json file into JSON.
fn read_config_json(
    resolver: &destack_resolver::Resolver,
    path: &Path,
) -> super::CommandResult<serde_json::Value> {
    let content = resolver
        .fs
        .read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(serde_json::from_str(&content).map_err(|error| format!("invalid dsconfig: {error}"))?)
}
