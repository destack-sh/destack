use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the info command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandInfoOptions {
    /// Whether to include all workspace packages.
    pub all: bool,
}

/// Workspace info for info command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfoWorkspace {
    /// Workspace root path.
    pub root: String,
    /// Workspace kind label.
    pub kind: String,
    /// Workspace package paths.
    pub packages: Vec<String>,
}

/// Target info for info command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfoTarget {
    /// Target name.
    pub name: String,
    /// Target emit format.
    pub emit: String,
    /// Target runtime.
    pub runtime: String,
    /// Target platform.
    pub platform: String,
    /// Output directory.
    pub out_dir: String,
    /// Output file when present.
    pub out_file: Option<String>,
    /// Package directory when present.
    pub package_dir: Option<String>,
}

/// Payload for info command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfoPayload {
    /// Workspace metadata.
    pub workspace: CommandInfoWorkspace,
    /// Resolved destack.json path.
    pub manifest: Option<String>,
    /// Targets for the active package.
    pub targets: Option<Vec<CommandInfoTarget>>,
    /// Targets for all workspace packages.
    pub workspace_targets: Option<Vec<CommandInfoTarget>>,
}

impl CommandContext<'_> {
    /// Execute an info command.
    pub(super) fn run_info_command(
        &mut self,
        options: &CommandInfoOptions,
    ) -> CommandResult<CommandOutcome> {
        // build workspace snapshot
        let revision = self.revision()?;
        let workspace = self
            .repository
            .workspace(revision)
            .map_err(|error| format!("failed to derive workspace: {error}"))?;
        let package_roots: Vec<String> = self
            .repository
            .package_roots(revision)
            .map_err(|error| format!("failed to derive workspace package roots: {error}"))?
            .iter()
            .map(|path| path.display().to_string())
            .collect();

        // resolve manifest from cwd
        let manifest_path = if self.common.manifest_path.is_some() {
            Some(self.resolve_destack_config_path(self.common.manifest_path.as_deref())?)
        } else {
            self.find_destack_config(self.session.cwd())
        };
        let config = manifest_path
            .as_ref()
            .and_then(|path| self.load_destack_config(path).ok());

        // load workspace configs when requested
        let workspace_configs = if options.all {
            self.load_workspace_configs(revision).ok()
        } else {
            None
        };

        // derive target summaries
        let targets = config.as_ref().map(|config| {
            config
                .targets
                .iter()
                .map(|(name, target)| CommandInfoTarget {
                    name: name.clone(),
                    emit: format!("{:?}", target.emit),
                    runtime: format!("{:?}", target.runtime),
                    platform: format!("{:?}", target.platform),
                    out_dir: target.out_dir.display().to_string(),
                    out_file: target
                        .out_file
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    package_dir: None,
                })
                .collect::<Vec<_>>()
        });

        // derive workspace target summaries
        let workspace_targets = workspace_configs.as_ref().map(|configs| {
            configs
                .iter()
                .flat_map(|config| {
                    let package_dir = config.directory.display().to_string();

                    config
                        .targets
                        .iter()
                        .map(move |(name, target)| CommandInfoTarget {
                            name: name.clone(),
                            emit: format!("{:?}", target.emit),
                            runtime: format!("{:?}", target.runtime),
                            platform: format!("{:?}", target.platform),
                            out_dir: target.out_dir.display().to_string(),
                            out_file: target
                                .out_file
                                .as_ref()
                                .map(|path| path.display().to_string()),
                            package_dir: Some(package_dir.clone()),
                        })
                })
                .collect::<Vec<_>>()
        });

        let payload = CommandInfoPayload {
            workspace: CommandInfoWorkspace {
                root: workspace.root.display().to_string(),
                kind: format!("{:?}", workspace.kind),
                packages: package_roots,
            },
            manifest: manifest_path
                .as_ref()
                .map(|path| path.display().to_string()),
            targets,
            workspace_targets,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid info payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
    }
}
