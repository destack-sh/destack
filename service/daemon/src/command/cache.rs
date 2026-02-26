use std::path::{Path, PathBuf};

use destack_source::DiagnosticCollection;
use destack_workspace::{CacheScope, DsConfig, resolve_cache_root_for_scope};
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

        // resolve dsconfigs based on scope
        let dsconfigs = if options.all_packages {
            self.load_workspace_dsconfigs(&workspace)?
        } else {
            let dsconfig_path = self.resolve_dsconfig_path(self.common.config_path.as_deref())?;
            vec![self.load_dsconfig(&dsconfig_path)?]
        };

        // collect cache locations
        let mut entries = Vec::new();
        if dsconfigs.is_empty() {
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
            for dsconfig in dsconfigs {
                let cache_dir = resolve_cache_dir(
                    self.common.cache_dir.as_ref(),
                    Some(&dsconfig),
                    &workspace.root,
                    &self.program.cwd,
                );
                let source = cache_source_label(self.common.cache_dir.as_ref(), &dsconfig);
                entries.push(CommandCacheEntry {
                    dir: cache_dir.display().to_string(),
                    source: source.to_string(),
                    package_dir: Some(dsconfig.directory.display().to_string()),
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
    dsconfig: Option<&DsConfig>,
    workspace_root: &Path,
    cwd: &Path,
) -> PathBuf {
    if let Some(cache_dir) = cache_override {
        if cache_dir.is_absolute() {
            return cache_dir.clone();
        }
        return cwd.join(cache_dir);
    }

    if let Some(dsconfig) = dsconfig {
        return resolve_cache_root_for_scope(
            &dsconfig.directory,
            dsconfig.options.cache.dir.as_deref(),
            dsconfig.options.cache.scope,
        );
    }

    resolve_cache_root_for_scope(workspace_root, None, CacheScope::Workspace)
}

/// Render a cache source label for reporting.
fn cache_source_label(cache_override: Option<&PathBuf>, dsconfig: &DsConfig) -> &'static str {
    if cache_override.is_some() {
        "override"
    } else if dsconfig.options.cache.dir.is_some() {
        "dsconfig"
    } else {
        "default"
    }
}
