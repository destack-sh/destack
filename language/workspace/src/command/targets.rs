use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_repository::{DestackFile, Target, TraceView};
use tspp_serde::Reflect;
use tspp_source::DiagnosticCollection;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the targets command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct TargetsOptions {
    /// Whether to list targets for all packages.
    pub all: bool,
}

/// Target entry shown by workspace discovery commands.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TargetEntry {
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
    /// Whether this is the package default target.
    pub is_default: bool,
    /// The owning package directory, when workspace-wide output is requested.
    pub package_dir: Option<String>,
}

/// Payload for targets command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TargetsPayload {
    /// List of target entries.
    pub targets: Vec<TargetEntry>,
}

/// Request to return configured targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
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
        let revision = self.revision();
        // resolve config selection
        let configs = if options.all {
            self.workspace_configs(revision)?
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
            entries.extend(TargetEntry::for_config(config, options.all));
        }

        if entries.is_empty() {
            return Err("no targets found".to_string().into());
        }

        let payload = TargetsPayload { targets: entries };

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}

impl TargetEntry {
    /// Return target entries for one package config.
    pub(crate) fn for_config(config: &DestackFile, include_package: bool) -> Vec<Self> {
        config
            .targets
            .iter()
            .map(|(name, target)| Self::from_target(name, target, config, include_package))
            .collect()
    }

    /// Return one target entry.
    fn from_target(
        name: &str,
        target: &Target,
        config: &DestackFile,
        include_package: bool,
    ) -> Self {
        let package_dir = include_package.then(|| config.directory.display().to_string());
        let is_default = config.default_target.as_deref() == Some(name);

        Self {
            name: name.to_string(),
            emit: format!("{:?}", target.output),
            runtime: format!("{:?}", target.runtime()),
            platform: format!("{:?}", target.platform),
            out_dir: target.destination.directory.display().to_string(),
            out_file: target
                .destination
                .file
                .as_ref()
                .map(|p| p.display().to_string()),
            is_default,
            package_dir,
        }
    }
}
