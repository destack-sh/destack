use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the targets command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandTargetsOptions {
    /// Whether to list targets for all packages.
    pub all: bool,
}

/// Target entry for targets command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTargetsEntry {
    /// The target name.
    pub name: String,
    /// The emit format.
    pub emit: String,
    /// The runtime environment.
    pub runtime: String,
    /// The target platform.
    pub platform: String,
    /// The output directory.
    pub out_dir: String,
    /// The output file path, when applicable.
    pub out_file: Option<String>,
    /// The default target name for the package.
    pub default_target: Option<String>,
    /// The owning package directory.
    pub package_dir: String,
}

/// Payload for targets command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTargetsPayload {
    /// List of target entries.
    pub targets: Vec<CommandTargetsEntry>,
}

impl CommandContext<'_> {
    /// Execute a targets command.
    pub(super) fn run_targets_command(
        &mut self,
        options: &CommandTargetsOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve workspace context
        let revision = self.revision()?;
        // resolve config selection
        let configs = if options.all {
            self.load_workspace_configs(revision)?
        } else {
            let config_path =
                self.resolve_destack_config_path(self.common.manifest_path.as_deref())?;
            vec![self.load_destack_config(&config_path)?]
        };

        if configs.is_empty() {
            return Err("no targets found".to_string().into());
        }

        // collect target details
        let mut entries = Vec::new();
        for config in &configs {
            let options = config;
            let default_target = options.default_target.clone();
            for (name, target) in &options.targets {
                entries.push(CommandTargetsEntry {
                    name: name.clone(),
                    emit: format!("{:?}", target.emit),
                    runtime: format!("{:?}", target.runtime),
                    platform: format!("{:?}", target.platform),
                    out_dir: target.out_dir.display().to_string(),
                    out_file: target.out_file.as_ref().map(|p| p.display().to_string()),
                    default_target: default_target.clone(),
                    package_dir: config.directory.display().to_string(),
                });
            }
        }

        if entries.is_empty() {
            return Err("no targets found".to_string().into());
        }

        let payload = CommandTargetsPayload { targets: entries };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid targets payload: {error}"))?;
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
    }
}
