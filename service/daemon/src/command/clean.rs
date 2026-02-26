use destack_source::DiagnosticCollection;
use destack_workspace::DsConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::cache::resolve_cache_dir;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the clean command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandCleanOptions {
    /// Optional directory override.
    pub dir: Option<PathBuf>,
    /// Remove build output directories.
    pub dist: bool,
    /// Remove cache directories.
    pub cache: bool,
    /// Remove all build outputs and caches.
    pub all: bool,
    /// Clean all packages in the workspace.
    pub all_packages: bool,
}

/// Payload for clean command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCleanPayload {
    /// Removed paths or candidate paths for dry runs.
    pub removed: Vec<String>,
    /// Errors encountered during removal.
    pub errors: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
}

impl CommandContext<'_> {
    /// Execute a clean command.
    pub(super) fn run_clean_command(
        &mut self,
        root: &Path,
        options: &CommandCleanOptions,
    ) -> super::CommandResult<CommandOutcome> {
        let cwd = options.dir.as_deref().unwrap_or(root);
        let fs = self.daemon.session.fs.clone();

        // decide which outputs to clean
        let clean_dist = options.dist || options.all || !options.cache;
        let clean_cache = options.cache || options.all;

        // resolve workspace context
        let workspace = self.daemon.session.workspace_snapshot();

        // resolve dsconfigs based on scope
        let dsconfigs = if options.all_packages {
            self.load_workspace_dsconfigs(&workspace)?
        } else {
            match self.resolve_dsconfig_path(self.common.config_path.as_deref()) {
                Ok(path) => vec![self.load_dsconfig(&path)?],
                Err(error) => {
                    if clean_dist {
                        return Err(error);
                    }
                    Vec::new()
                }
            }
        };

        // collect paths for removal
        let mut paths = HashSet::new();
        if clean_dist {
            for dsconfig in &dsconfigs {
                collect_output_paths(dsconfig, &mut paths);
            }
        }
        if clean_cache {
            if dsconfigs.is_empty() {
                let cache_dir =
                    resolve_cache_dir(self.common.cache_dir.as_ref(), None, &workspace.root, cwd);
                paths.insert(cache_dir);
            } else {
                for dsconfig in &dsconfigs {
                    let cache_dir = resolve_cache_dir(
                        self.common.cache_dir.as_ref(),
                        Some(dsconfig),
                        &workspace.root,
                        cwd,
                    );
                    paths.insert(cache_dir);
                }
            }
        }

        // delete selected paths
        let mut removed = Vec::new();
        let mut errors = Vec::new();
        for path in paths {
            if self.common.dry_run {
                removed.push(path.display().to_string());
                continue;
            }

            match fs.remove_path(&path) {
                Ok(true) => removed.push(path.display().to_string()),
                Ok(false) => {}
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }

        // emit output messages
        if self.common.dry_run {
            for path in &removed {
                self.output
                    .push_stdout(format!("would remove {path}\n").into_bytes());
            }
        } else {
            for path in &removed {
                self.output
                    .push_stdout(format!("removed {path}\n").into_bytes());
            }
        }
        for error in &errors {
            self.output.push_stderr(format!("{error}\n").into_bytes());
        }

        let exit_code = if errors.is_empty() { 0 } else { 1 };
        let payload = CommandCleanPayload {
            removed,
            errors,
            dry_run: self.common.dry_run,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid clean payload: {error}"))?;
        Ok(
            CommandOutcome::new(DiagnosticCollection::default(), exit_code, 0, 0, 0, None)
                .with_data(data),
        )
    }
}

/// Collect output paths for a dsconfig.
fn collect_output_paths(dsconfig: &DsConfig, paths: &mut HashSet<PathBuf>) {
    if let Some(out_dir) = dsconfig.options.compiler.out_dir.as_ref() {
        paths.insert(resolve_path(out_dir, &dsconfig.directory));
    }
    if let Some(declaration_dir) = dsconfig.options.compiler.declaration_dir.as_ref() {
        paths.insert(resolve_path(declaration_dir, &dsconfig.directory));
    }

    for target in dsconfig.options.targets.values() {
        let out_dir = resolve_path(&target.out_dir, &dsconfig.directory);
        paths.insert(out_dir);

        if let Some(out_file) = target.out_file.as_ref() {
            paths.insert(resolve_path(out_file, &dsconfig.directory));
        }
        if let Some(declaration_dir) = target.declaration_dir.as_ref() {
            paths.insert(resolve_path(declaration_dir, &dsconfig.directory));
        }
    }
}

/// Resolve a path relative to the provided root.
fn resolve_path(path: &Path, root: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
