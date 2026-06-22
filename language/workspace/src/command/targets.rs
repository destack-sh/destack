use std::path::PathBuf;

use destack_serde::Schema;
use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the targets command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema, Default)]
pub struct TargetsOptions {
    /// Whether to list targets for all packages.
    pub all: bool,
}

/// Target entry for targets command output.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TargetsEntry {
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
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TargetsPayload {
    /// List of target entries.
    pub targets: Vec<TargetsEntry>,
}

/// Request to return configured targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct TargetsInput {
    /// Revision selected for this targets request.
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
    /// Whether to list targets for all packages.
    pub all: bool,
}

impl_command_input_options!(TargetsInput { all: false });

impl CommandContext<'_> {
    /// Execute a targets command.
    pub(crate) fn run_targets_command(
        &mut self,
        options: &TargetsOptions,
    ) -> CommandResult<CommandOutcome<TargetsPayload>> {
        // resolve workspace context
        let revision = self.revision()?;
        // resolve config selection
        let configs = if options.all {
            self.load_workspace_configs(revision)?
        } else {
            let config_path = self.resolve_destack_config_path(self.common.manifest.as_deref())?;
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
                entries.push(TargetsEntry {
                    name: name.clone(),
                    emit: format!("{:?}", target.emit),
                    runtime: format!("{:?}", target.runtime()),
                    platform: format!("{:?}", target.platform),
                    out_dir: target.output.directory.display().to_string(),
                    out_file: target.output.file.as_ref().map(|p| p.display().to_string()),
                    default_target: default_target.clone(),
                    package_dir: config.directory.display().to_string(),
                });
            }
        }

        if entries.is_empty() {
            return Err("no targets found".to_string().into());
        }

        let payload = TargetsPayload { targets: entries };

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}
