use std::path::PathBuf;

use destack_serde::Reflect;
use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the cache command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct CacheOptions;

/// Cache entry payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct CacheEntry {
    /// Cache directory path.
    pub directory: String,
    /// Cache kind.
    pub kind: String,
}

/// Cache payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct CachePayload {
    /// Cache entries for the workspace.
    pub caches: Vec<CacheEntry>,
}

/// Request to return cache locations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CacheInput {
    /// Revision selected for this cache request.
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
}

impl_command_input_options!(CacheInput {});

impl CommandContext<'_> {
    /// Execute one cache command.
    pub(crate) fn run_cache_command(
        &mut self,
        _options: &CacheOptions,
    ) -> CommandResult<CommandOutcome<CachePayload>> {
        // read the repository layout resolved at launch
        let layout = self.repository.layout();

        // build the cache payload
        let payload = CachePayload {
            caches: vec![CacheEntry {
                directory: layout.workspace_cache.display().to_string(),
                kind: "workspace".to_string(),
            }],
        };

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(payload))
    }
}
