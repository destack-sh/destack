use std::path::{Path, PathBuf};

use destack_source::DiagnosticCollection;
use destack_workspace::{CacheScope, Destack, resolve_cache_root_for_scope};
use serde::{Deserialize, Serialize};

use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the cache command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandCacheOptions {
    /// Whether to list caches for all packages.
    pub all_packages: bool,
}

/// Cache entry payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCacheEntry {
    /// Cache directory path.
    pub dir: String,
    /// Source of the cache location.
    pub source: String,
    /// Package directory when available.
    pub package_dir: Option<String>,
}

/// Cache payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCachePayload {
    /// Cache entries for the workspace or packages.
    pub caches: Vec<CommandCacheEntry>,
}

impl CommandContext<'_> {
    /// Execute a cache command.
    pub(super) fn run_cache_command(
        &mut self,
        options: &CommandCacheOptions,
    ) -> super::CommandResult<CommandOutcome> {
        let workspace = self.daemon.session.workspace_snapshot();

        // resolve configs based on scope
        let destack_configs = if options.all_packages {
            self.load_workspace_configs(&workspace)?
        } else {
            let config_path =
                self.resolve_destack_config_path(self.common.config_path.as_deref())?;
            vec![self.load_destack_config(&config_path)?]
        };

        // collect cache locations
        let mut entries = Vec::new();
        if destack_configs.is_empty() {
            let cache_dir = resolve_cache_dir(
                self.common.cache_dir.as_ref(),
                None,
                &workspace.root,
                &self.program.cwd,
            );
            entries.push(CommandCacheEntry {
                dir: cache_dir.display().to_string(),
                source: "default".to_string(),
                package_dir: None,
            });
        } else {
            for config in destack_configs {
                let cache_dir = resolve_cache_dir(
                    self.common.cache_dir.as_ref(),
                    Some(&config),
                    &workspace.root,
                    &self.program.cwd,
                );
                let source = cache_source_label(self.common.cache_dir.as_ref(), &config);
                entries.push(CommandCacheEntry {
                    dir: cache_dir.display().to_string(),
                    source: source.to_string(),
                    package_dir: Some(config.directory.display().to_string()),
                });
            }
        }

        let payload = CommandCachePayload { caches: entries };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid cache payload: {error}"))?;
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0, None).with_data(data))
    }
}

/// Resolve the cache directory for a workspace.
pub(super) fn resolve_cache_dir(
    cache_override: Option<&PathBuf>,
    config: Option<&Destack>,
    workspace_root: &Path,
    cwd: &Path,
) -> PathBuf {
    if let Some(cache_dir) = cache_override {
        if cache_dir.is_absolute() {
            return cache_dir.clone();
        }
        return cwd.join(cache_dir);
    }

    if let Some(config) = config {
        return resolve_cache_root_for_scope(
            &config.directory,
            config.options.cache.dir.as_deref(),
            config.options.cache.scope,
        );
    }

    resolve_cache_root_for_scope(workspace_root, None, CacheScope::Workspace)
}

/// Render a cache source label for reporting.
fn cache_source_label(cache_override: Option<&PathBuf>, config: &Destack) -> &'static str {
    if cache_override.is_some() {
        "override"
    } else if config.options.cache.dir.is_some() {
        "destack"
    } else {
        "default"
    }
}
