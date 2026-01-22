use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

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
    /// Target output format.
    pub output: String,
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
    /// Resolved dsconfig path.
    pub dsconfig: Option<String>,
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
    ) -> Result<CommandOutcome, String> {
        // build workspace snapshot
        let workspace = self.daemon.session.workspace_snapshot();
        let package_paths: Vec<String> = workspace
            .package_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect();

        // resolve dsconfig from cwd
        let dsconfig_path = if self.common.config_path.is_some() {
            Some(self.resolve_dsconfig_path(self.common.config_path.as_deref())?)
        } else {
            self.find_dsconfig(&self.program.cwd)
        };
        let dsconfig = dsconfig_path
            .as_ref()
            .and_then(|path| self.load_dsconfig(path).ok());

        // load workspace configs when requested
        let workspace_configs = if options.all {
            self.load_workspace_dsconfigs(&workspace).ok()
        } else {
            None
        };

        // derive target summaries
        let targets = dsconfig.as_ref().map(|config| {
            config
                .options
                .targets
                .iter()
                .map(|(name, target)| CommandInfoTarget {
                    name: name.clone(),
                    output: format!("{:?}", target.output),
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
                    config
                        .options
                        .targets
                        .iter()
                        .map(|(name, target)| CommandInfoTarget {
                            name: name.clone(),
                            output: format!("{:?}", target.output),
                            runtime: format!("{:?}", target.runtime),
                            platform: format!("{:?}", target.platform),
                            out_dir: target.out_dir.display().to_string(),
                            out_file: target
                                .out_file
                                .as_ref()
                                .map(|path| path.display().to_string()),
                            package_dir: Some(config.directory.display().to_string()),
                        })
                })
                .collect::<Vec<_>>()
        });

        let payload = CommandInfoPayload {
            workspace: CommandInfoWorkspace {
                root: workspace.root.display().to_string(),
                kind: format!("{:?}", workspace.kind),
                packages: package_paths,
            },
            dsconfig: dsconfig_path
                .as_ref()
                .map(|path| path.display().to_string()),
            targets,
            workspace_targets,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid info payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0, None).with_data(data))
    }
}
