use std::path::{Path, PathBuf};
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_repository::{RegistryAuthentication, TraceView};
use tspp_source::DiagnosticCollection;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the settings command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct SettingsOptions;

/// Request to return resolved settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SettingsInput {
    /// Revision selected for this settings request.
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
}

impl_command_input_options!(SettingsInput {});

/// Payload for settings command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct SettingsPayload {
    /// Machine-local Destack home.
    pub home: String,
    /// Package directory.
    pub packages: String,
    /// Maximum package directory size in bytes before pruning is requested.
    pub package_maximum_bytes: Option<u64>,
    /// Machine-local cache directory.
    pub cache: String,
    /// Maximum cache size in bytes before pruning is requested.
    pub cache_maximum_bytes: Option<u64>,
    /// Workspace-owned vendor directory.
    pub vendor: String,
    /// Default registry name.
    pub registry: Option<String>,
    /// Known registries.
    pub registries: Vec<SettingsRegistry>,
    /// Network settings for package and update commands.
    pub network: SettingsNetwork,
}

/// Registry settings for command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct SettingsRegistry {
    /// Registry name.
    pub name: String,
    /// Registry URL.
    pub url: String,
    /// Redacted authentication shape.
    pub authentication: SettingsRegistryAuthentication,
}

/// Redacted registry authentication shape.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub enum SettingsRegistryAuthentication {
    /// No registry authentication.
    None,
    /// Inline token authentication.
    Token,
    /// Token read from one environment variable.
    TokenFromEnvironment {
        /// Environment variable name.
        variable: String,
    },
    /// Credentials printed by one process command.
    Command {
        /// Program to run.
        program: String,
    },
}

/// Network settings for command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct SettingsNetwork {
    /// Whether network access should be disabled by default.
    pub offline: bool,
    /// Whether a proxy is configured.
    pub has_proxy: bool,
    /// Request timeout in milliseconds.
    pub timeout_milliseconds: Option<u64>,
    /// Number of retries for transient network failures.
    pub retry_count: Option<u32>,
    /// Maximum concurrent network requests.
    pub concurrency: Option<u32>,
}

impl CommandContext<'_> {
    /// Execute one settings command.
    pub(crate) fn run_settings_command(
        &mut self,
        _options: &SettingsOptions,
    ) -> CommandResult<CommandOutcome<SettingsPayload>> {
        // read the repository layout resolved at launch
        let layout = self.repository.layout();
        let settings = self.repository.settings();

        // resolve workspace-owned settings
        let revision = self.revision();
        let vendor = self
            .repository
            .destack_for_workspace(revision)
            .map_err(|error| format!("failed to load workspace manifest: {error}"))?
            .map(|manifest| resolve_workspace_path(self.repository.path(), &manifest.vendor.path))
            .unwrap_or_else(|| layout.vendor.clone());

        // redact registry settings for command output
        let registries = settings
            .registries
            .iter()
            .map(|(name, registry)| SettingsRegistry {
                name: name.clone(),
                url: registry.url.clone(),
                authentication: redact_authentication(&registry.authentication),
            })
            .collect();

        // build the settings payload
        let payload = SettingsPayload {
            home: layout.home.display().to_string(),
            packages: layout.packages.display().to_string(),
            package_maximum_bytes: settings.packages.maximum_bytes,
            cache: layout.cache.display().to_string(),
            cache_maximum_bytes: settings.cache.maximum_bytes,
            vendor: vendor.display().to_string(),
            registry: settings.registry.clone(),
            registries,
            network: SettingsNetwork {
                offline: settings.network.offline,
                has_proxy: settings.network.proxy.is_some(),
                timeout_milliseconds: settings.network.timeout_milliseconds,
                retry_count: settings.network.retry_count,
                concurrency: settings.network.concurrency,
            },
        };

        // serialize command payload
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}

/// Resolve a workspace path against the workspace root.
fn resolve_workspace_path(root: &Path, path: &Path) -> PathBuf {
    // return absolute paths unchanged
    if path.is_absolute() {
        path.to_path_buf()
    }
    // resolve relative paths against the workspace
    else {
        root.join(path)
    }
}

/// Redact registry authentication secrets for display.
fn redact_authentication(
    authentication: &RegistryAuthentication,
) -> SettingsRegistryAuthentication {
    match authentication {
        RegistryAuthentication::None => SettingsRegistryAuthentication::None,
        RegistryAuthentication::Token { .. } => SettingsRegistryAuthentication::Token,
        RegistryAuthentication::TokenFromEnvironment { variable } => {
            SettingsRegistryAuthentication::TokenFromEnvironment {
                variable: variable.clone(),
            }
        }
        RegistryAuthentication::Command { program, .. } => {
            SettingsRegistryAuthentication::Command {
                program: program.clone(),
            }
        }
    }
}
