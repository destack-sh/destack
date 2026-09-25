use std::path::PathBuf;
use std::process::Command;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_repository::{DestackFile, TraceView};
use tspp_source::DiagnosticCollection;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the doctor command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct DoctorOptions {
    /// Whether to run extended checks.
    pub full: bool,
}

/// Request to return workspace health information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DoctorInput {
    /// Revision selected for this doctor request.
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
    /// Whether to run extended checks.
    pub full: bool,
}

impl_command_input_options!(DoctorInput { full: false });

/// Tool probe entry for doctor output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DoctorTool {
    /// Tool name.
    pub name: String,
    /// Tool version string.
    pub version: Option<String>,
    /// Tool probe status.
    pub status: DoctorToolStatus,
}

/// Status values for tool detection.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "snake_case")]
pub enum DoctorToolStatus {
    /// Tool responded successfully.
    Available,
    /// Tool was not found on the system.
    Missing,
    /// Tool returned an error response.
    Error,
}

/// Workspace details for doctor output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DoctorWorkspace {
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
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DoctorPayload {
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
    pub workspace: DoctorWorkspace,
    /// Resolved destack.json path.
    pub manifest: Option<String>,
    /// Manifest extends entries.
    pub extends: Option<Vec<String>>,
    /// Default target name.
    pub default_target: Option<String>,
    /// Target count.
    pub target_count: usize,
    /// Target names when requested.
    pub targets: Option<Vec<String>>,
    /// Tool checks when requested.
    pub tools: Option<Vec<DoctorTool>>,
    /// Warning list.
    pub warnings: Option<Vec<String>>,
}

impl CommandContext<'_> {
    /// Execute a doctor command.
    pub(crate) fn run_doctor_command(
        &mut self,
        options: &DoctorOptions,
    ) -> CommandResult<CommandOutcome<DoctorPayload>> {
        // collect static environment info
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let available_parallelism = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);

        // resolve workspace context
        let revision = self.revision();
        let workspace = self
            .repository
            .root(revision)
            .map_err(|error| format!("failed to derive workspace: {error}"))?;

        // resolve manifest
        let manifest = if self.common.manifest.is_some() {
            self.resolve_destack_config_path(self.common.manifest.as_deref())
                .ok()
        } else {
            self.find_destack_config(&self.cwd)
        };
        let config = manifest
            .as_ref()
            .and_then(|path| self.load_destack_config(path).ok());

        // collect manifest warnings
        let mut warnings = Vec::new();
        if manifest.is_none() {
            warnings.push("destack.json not found".to_string());
        }

        let mut target_names = Vec::new();
        let mut default_target = None;
        let mut extends = Vec::new();
        if let Some(config) = config.as_ref() {
            let options = config;
            target_names = options.targets.keys().cloned().collect();
            default_target = options.default_target.clone();
            extends = list_extends(config);
            if let Some(default_target) = default_target.as_ref()
                && !options.targets.contains_key(default_target)
            {
                warnings.push(format!("default target '{default_target}' is not defined"));
            }
            if options.targets.is_empty() {
                warnings.push("no targets configured".to_string());
            }
        }

        // collect command info only when requested
        let tools = if options.full {
            vec![
                probe_tool("rustc", &["--version"]),
                probe_tool("cargo", &["--version"]),
                probe_tool("deno", &["--version"]),
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

        let payload = DoctorPayload {
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            cwd: self.cwd.display().to_string(),
            os: os.to_string(),
            arch: arch.to_string(),
            workers: self.workspace.session().executor().worker_count() as u64,
            available_parallelism: u64::try_from(available_parallelism).unwrap_or(u64::MAX),
            workspace: DoctorWorkspace {
                root: workspace.root.display().to_string(),
                kind: format!("{:?}", workspace.kind),
                package_count: package_roots.len(),
                packages: if options.full {
                    Some(package_roots)
                } else {
                    None
                },
            },
            manifest: manifest.as_ref().map(|path| path.display().to_string()),
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
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}

/// Tool probing result for doctor output.
/// Query a tool for its version string.
fn probe_tool(name: &'static str, args: &[&str]) -> DoctorTool {
    let output = Command::new(name).args(args).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let version = parse_version_output(&output.stdout, &output.stderr);
                DoctorTool {
                    name: name.to_string(),
                    version,
                    status: DoctorToolStatus::Available,
                }
            } else {
                DoctorTool {
                    name: name.to_string(),
                    version: None,
                    status: DoctorToolStatus::Error,
                }
            }
        }
        Err(error) => {
            let status = if error.kind() == std::io::ErrorKind::NotFound {
                DoctorToolStatus::Missing
            } else {
                DoctorToolStatus::Error
            };
            DoctorTool {
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
fn list_extends(config: &DestackFile) -> Vec<String> {
    config.extends.iter().cloned().collect()
}
