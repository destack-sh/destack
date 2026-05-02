use std::path::{Path, PathBuf};

use destack_source::DiagnosticCollection;
use destack_workspace::resolve_cache_root;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the cache command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandCacheOptions;

/// Cache entry payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCacheEntry {
    /// Cache directory path.
    pub directory: String,
    /// Source of the cache location.
    pub source: String,
}

/// Cache payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCachePayload {
    /// Cache entries for the workspace.
    pub caches: Vec<CommandCacheEntry>,
}

impl CommandContext<'_> {
    /// Execute one cache command.
    pub(super) fn run_cache_command(
        &mut self,
        _options: &CommandCacheOptions,
    ) -> CommandResult<CommandOutcome> {
        let revision = self.revision()?;
        let workspace = self
            .daemon
            .repository
            .workspace(revision)
            .map_err(|error| format!("failed to derive workspace: {error}"))?;

        // resolve the workspace cache location
        let cache_directory = resolve_cache_directory(
            self.common.cache_dir.as_ref(),
            &workspace.root,
            self.repository.workspace_root(),
        );
        let source = cache_source_label(self.common.cache_dir.as_ref());

        let payload = CommandCachePayload {
            caches: vec![CommandCacheEntry {
                directory: cache_directory.display().to_string(),
                source: source.to_string(),
            }],
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid cache payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
    }
}

/// Resolve one cache directory for one workspace.
pub(super) fn resolve_cache_directory(
    cache_override: Option<&PathBuf>,
    workspace_root: &Path,
    cwd: &Path,
) -> PathBuf {
    if let Some(cache_override) = cache_override {
        if cache_override.is_absolute() {
            return cache_override.clone();
        }

        return cwd.join(cache_override);
    }

    resolve_cache_root(workspace_root, None)
}

/// Render one cache source label for reporting.
fn cache_source_label(cache_override: Option<&PathBuf>) -> &'static str {
    if cache_override.is_some() {
        "override"
    } else {
        "default"
    }
}
