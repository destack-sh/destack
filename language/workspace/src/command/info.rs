use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tspp_repository::TraceView;
use tspp_serde::Reflect;
use tspp_source::DiagnosticCollection;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
use super::targets::TargetEntry;
/// Options for the info command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct InfoOptions {
    /// Whether to include all workspace packages.
    pub all: bool,
}

/// Workspace info for info command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct InfoWorkspace {
    /// Workspace root path.
    pub root: String,
    /// Workspace kind label.
    pub kind: String,
    /// Workspace package paths.
    pub packages: Vec<String>,
}

/// Payload for info command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct InfoPayload {
    /// Workspace metadata.
    pub workspace: InfoWorkspace,
    /// Resolved destack.json path.
    pub manifest: Option<String>,
    /// Targets for the active package.
    pub targets: Option<Vec<TargetEntry>>,
    /// Targets for all workspace packages.
    pub workspace_targets: Option<Vec<TargetEntry>>,
}

/// Request to return workspace information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InfoInput {
    /// Revision selected for this info request.
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
    /// Whether to include all workspace packages.
    pub all: bool,
}

impl_command_input_options!(InfoInput { all: false });

impl CommandContext<'_> {
    /// Execute an info command.
    pub(crate) fn run_info_command(
        &mut self,
        options: &InfoOptions,
    ) -> CommandResult<CommandOutcome<InfoPayload>> {
        // build workspace snapshot
        let revision = self.revision();
        let workspace = self
            .repository
            .root(revision)
            .map_err(|error| format!("failed to derive workspace: {error}"))?;
        let package_roots: Vec<String> = self
            .repository
            .package_roots(revision)
            .map_err(|error| format!("failed to derive workspace package roots: {error}"))?
            .iter()
            .map(|path| path.display().to_string())
            .collect();

        // resolve manifest from cwd
        let manifest = if self.common.manifest.is_some() {
            Some(self.resolve_destack_config_path(self.common.manifest.as_deref())?)
        } else {
            self.find_destack_config(&self.cwd)
        };
        let config = manifest
            .as_ref()
            .and_then(|path| self.load_destack_config(path).ok());

        // load workspace configs when requested
        let workspace_configs = if options.all {
            self.workspace_configs(revision).ok()
        } else {
            None
        };

        // derive target entries
        let targets = config
            .as_ref()
            .map(|config| TargetEntry::for_config(config, false));

        // derive workspace target entries
        let workspace_targets = workspace_configs.as_ref().map(|configs| {
            configs
                .iter()
                .flat_map(|config| TargetEntry::for_config(config, true))
                .collect::<Vec<_>>()
        });

        let payload = InfoPayload {
            workspace: InfoWorkspace {
                root: workspace.root.display().to_string(),
                kind: format!("{:?}", workspace.kind),
                packages: package_roots,
            },
            manifest: manifest.as_ref().map(|path| path.display().to_string()),
            targets,
            workspace_targets,
        };
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}
