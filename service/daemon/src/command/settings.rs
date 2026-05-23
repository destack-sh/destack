use std::path::{Path, PathBuf};

use destack_source::DiagnosticCollection;
use destack_workspace::RegistryAuthentication;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the settings command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandSettingsOptions;

/// Payload for settings command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSettingsPayload {
    /// Machine-local Destack home.
    pub home: String,
    /// Package directory.
    pub packages: String,
    /// Maximum package directory size in bytes before pruning is requested.
    pub package_maximum_bytes: Option<u64>,
    /// Workspace-local cache and session directory.
    pub workspace_cache: String,
    /// Maximum cache size in bytes before pruning is requested.
    pub cache_maximum_bytes: Option<u64>,
    /// Workspace-owned vendor directory.
    pub vendor: String,
    /// Default registry name.
    pub registry: Option<String>,
    /// Known registries.
    pub registries: Vec<CommandSettingsRegistry>,
    /// Network settings for package and update commands.
    pub network: CommandSettingsNetwork,
}

/// Registry settings for command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSettingsRegistry {
    /// Registry name.
    pub name: String,
    /// Registry URL.
    pub url: String,
    /// Redacted authentication shape.
    pub authentication: CommandSettingsRegistryAuthentication,
}

/// Redacted registry authentication shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum CommandSettingsRegistryAuthentication {
    /// No registry authentication.
    None,
    /// Inline token authentication.
    Token,
    /// Token read from one environment variable.
    TokenFromEnvironment {
        /// Environment variable name.
        variable: String,
    },
    /// Credentials printed by one command.
    Command {
        /// Program to run.
        program: String,
    },
}

/// Network settings for command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSettingsNetwork {
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
    pub(super) fn run_settings_command(
        &mut self,
        _options: &CommandSettingsOptions,
    ) -> CommandResult<CommandOutcome> {
        // read the repository layout resolved at launch
        let layout = self.repository.layout();
        let settings = self.repository.settings();

        // resolve workspace-owned settings
        let revision = self.revision()?;
        let vendor = self
            .repository
            .destack_for_workspace(revision)
            .map_err(|error| format!("failed to load workspace manifest: {error}"))?
            .map(|manifest| {
                resolve_workspace_path(self.repository.workspace_root(), &manifest.vendor.path)
            })
            .unwrap_or_else(|| layout.vendor.clone());

        // redact registry settings for command output
        let registries = settings
            .registries
            .iter()
            .map(|(name, registry)| CommandSettingsRegistry {
                name: name.clone(),
                url: registry.url.clone(),
                authentication: redact_authentication(&registry.authentication),
            })
            .collect();

        // build the settings payload
        let payload = CommandSettingsPayload {
            home: layout.home.display().to_string(),
            packages: layout.packages.display().to_string(),
            package_maximum_bytes: settings.packages.maximum_bytes,
            workspace_cache: layout.workspace_cache.display().to_string(),
            cache_maximum_bytes: settings.cache.maximum_bytes,
            vendor: vendor.display().to_string(),
            registry: settings.registry.clone(),
            registries,
            network: CommandSettingsNetwork {
                offline: settings.network.offline,
                has_proxy: settings.network.proxy.is_some(),
                timeout_milliseconds: settings.network.timeout_milliseconds,
                retry_count: settings.network.retry_count,
                concurrency: settings.network.concurrency,
            },
        };

        // serialize command payload
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid settings payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
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
) -> CommandSettingsRegistryAuthentication {
    match authentication {
        RegistryAuthentication::None => CommandSettingsRegistryAuthentication::None,
        RegistryAuthentication::Token { .. } => CommandSettingsRegistryAuthentication::Token,
        RegistryAuthentication::TokenFromEnvironment { variable } => {
            CommandSettingsRegistryAuthentication::TokenFromEnvironment {
                variable: variable.clone(),
            }
        }
        RegistryAuthentication::Command { program, .. } => {
            CommandSettingsRegistryAuthentication::Command {
                program: program.clone(),
            }
        }
    }
}
