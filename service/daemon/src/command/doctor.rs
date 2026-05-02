use std::process::Command;

use destack_source::DiagnosticCollection;
use destack_workspace::{DestackDeclaration, ExtendsFieldJson};
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the doctor command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandDoctorOptions {
    /// Whether to run extended checks.
    pub full: bool,
}

/// Tool probe entry for doctor output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDoctorTool {
    /// Tool name.
    pub name: String,
    /// Tool version string.
    pub version: Option<String>,
    /// Tool probe status.
    pub status: CommandDoctorToolStatus,
}

/// Status values for tool detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandDoctorToolStatus {
    /// Tool responded successfully.
    Available,
    /// Tool was not found on the system.
    Missing,
    /// Tool returned an error response.
    Error,
}

/// Workspace details for doctor output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDoctorWorkspace {
    /// Workspace root path.
    pub root: String,
    /// Workspace kind label.
    pub kind: String,
    /// Number of packages.
    pub package_count: usize,
    /// Package paths when requested.
    pub packages: Option<Vec<String>>,
}

/// Payload for doctor command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDoctorPayload {
    /// CLI version string.
    pub cli_version: String,
    /// Working directory.
    pub cwd: String,
    /// Host operating system.
    pub os: String,
    /// Host architecture.
    pub arch: String,
    /// Worker count configured.
    pub workers: u64,
    /// Detected parallelism.
    pub available_parallelism: u64,
    /// Workspace metadata.
    pub workspace: CommandDoctorWorkspace,
    /// Resolved destack.json path.
    pub config: Option<String>,
    /// Config extends entries.
    pub extends: Option<Vec<String>>,
    /// Default target name.
    pub default_target: Option<String>,
    /// Target count.
    pub target_count: usize,
    /// Target names when requested.
    pub targets: Option<Vec<String>>,
    /// Tool checks when requested.
    pub tools: Option<Vec<CommandDoctorTool>>,
    /// Warning list.
    pub warnings: Option<Vec<String>>,
}

impl CommandContext<'_> {
    /// Execute a doctor command.
    pub(super) fn run_doctor_command(
        &mut self,
        options: &CommandDoctorOptions,
    ) -> CommandResult<CommandOutcome> {
        // collect static environment info
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let available_parallelism = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);

        // resolve workspace context
        let revision = self.revision()?;
        let workspace = self
            .daemon
            .repository
            .workspace(revision)
            .map_err(|error| format!("failed to derive workspace: {error}"))?;

        // resolve config
        let config_path = if self.common.config_path.is_some() {
            self.resolve_destack_config_path(self.common.config_path.as_deref())
                .ok()
        } else {
            self.find_destack_config(self.session.cwd())
        };
        let declaration = config_path
            .as_ref()
            .and_then(|path| self.load_destack_declaration(path).ok());

        // collect config warnings
        let mut warnings = Vec::new();
        if config_path.is_none() {
            warnings.push("destack.json not found".to_string());
        }

        let mut target_names = Vec::new();
        let mut default_target = None;
        let mut extends = Vec::new();
        if let Some(declaration) = declaration.as_ref() {
            let options = declaration.package_options();
            target_names = options.targets.keys().cloned().collect();
            default_target = options.default_target.clone();
            extends = list_extends(declaration);
            if let Some(default_target) = default_target.as_ref()
                && !options.targets.contains_key(default_target)
            {
                warnings.push(format!("default target '{default_target}' is not defined"));
            }
            if options.targets.is_empty() {
                warnings.push("no targets configured".to_string());
            }
        }

        // collect toolchain info only when requested
        let tools = if options.full {
            vec![
                probe_tool("rustc", &["--version"]),
                probe_tool("cargo", &["--version"]),
                probe_tool("bun", &["--version"]),
                probe_tool("node", &["--version"]),
            ]
        } else {
            Vec::new()
        };

        let package_roots: Vec<String> = self
            .repository
            .package_roots(revision)
            .map_err(|error| format!("failed to derive workspace package roots: {error}"))?
            .iter()
            .map(|path| path.display().to_string())
            .collect();

        let payload = CommandDoctorPayload {
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            cwd: self.session.cwd().display().to_string(),
            os: os.to_string(),
            arch: arch.to_string(),
            workers: self.daemon.worker_limit as u64,
            available_parallelism: u64::try_from(available_parallelism).unwrap_or(u64::MAX),
            workspace: CommandDoctorWorkspace {
                root: workspace.root.display().to_string(),
                kind: format!("{:?}", workspace.kind),
                package_count: package_roots.len(),
                packages: if options.full {
                    Some(package_roots)
                } else {
                    None
                },
            },
            config: config_path.as_ref().map(|path| path.display().to_string()),
            extends: if extends.is_empty() {
                None
            } else {
                Some(extends)
            },
            default_target,
            target_count: target_names.len(),
            targets: if options.full {
                Some(target_names)
            } else {
                None
            },
            tools: if options.full { Some(tools) } else { None },
            warnings: if warnings.is_empty() {
                None
            } else {
                Some(warnings)
            },
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid doctor payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
    }
}

/// Tool probing result for doctor output.
/// Query a tool for its version string.
fn probe_tool(name: &'static str, args: &[&str]) -> CommandDoctorTool {
    let output = Command::new(name).args(args).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let version = parse_version_output(&output.stdout, &output.stderr);
                CommandDoctorTool {
                    name: name.to_string(),
                    version,
                    status: CommandDoctorToolStatus::Available,
                }
            } else {
                CommandDoctorTool {
                    name: name.to_string(),
                    version: None,
                    status: CommandDoctorToolStatus::Error,
                }
            }
        }
        Err(error) => {
            let status = if error.kind() == std::io::ErrorKind::NotFound {
                CommandDoctorToolStatus::Missing
            } else {
                CommandDoctorToolStatus::Error
            };
            CommandDoctorTool {
                name: name.to_string(),
                version: None,
                status,
            }
        }
    }
}

/// Extract a version string from tool output.
fn parse_version_output(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    let output = String::from_utf8_lossy(stdout).to_string();
    if !output.trim().is_empty() {
        return Some(output.trim().to_string());
    }

    let output = String::from_utf8_lossy(stderr).to_string();
    if !output.trim().is_empty() {
        return Some(output.trim().to_string());
    }

    None
}

/// Collect extends entries for a config.
fn list_extends(declaration: &DestackDeclaration) -> Vec<String> {
    let mut entries = Vec::new();
    match &declaration.json.extends {
        Some(ExtendsFieldJson::Single(value)) => entries.push(value.clone()),
        Some(ExtendsFieldJson::Multiple(values)) => entries.extend(values.clone()),
        None => {}
    }
    entries
}
