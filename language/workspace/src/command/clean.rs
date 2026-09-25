use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tspp_repository::{ManifestFile, TraceView};
use tspp_serde::Reflect;
use tspp_source::DiagnosticCollection;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;

/// Options for the clean command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct CleanOptions {
    /// Optional directory override.
    pub dir: Option<PathBuf>,
    /// Clean all packages in the workspace.
    pub all_packages: bool,
}

/// Payload for clean command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct CleanPayload {
    /// Removed paths or candidate paths for dry runs.
    pub removed: Vec<String>,
    /// Errors encountered during removal.
    pub errors: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
}

/// Request to clean generated state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CleanInput {
    /// Revision selected for this clean request.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether package.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional manifest path override.
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
    /// Optional directory override.
    pub dir: Option<PathBuf>,
    /// Clean all packages in the workspace.
    pub all_packages: bool,
}

impl_command_input_options!(CleanInput {
    dir: None,
    all_packages: false,
});

impl CommandContext<'_> {
    /// Execute a clean command.
    pub(crate) fn run_clean_command(
        &mut self,
        options: &CleanOptions,
    ) -> CommandResult<CommandOutcome<CleanPayload>> {
        let file_system = self.repository.file_system().clone();

        // resolve workspace context
        let revision = self.revision();

        // resolve configs for output cleanup
        let manifests = if options.all_packages {
            self.workspace_configs(revision)?
        } else {
            let manifest = options.dir.as_deref().or(self.common.manifest.as_deref());
            let path = self.resolve_manifest_path(manifest)?;

            vec![self.load_manifest(&path)?]
        };

        // collect paths for removal
        let mut paths = BTreeSet::new();
        for config in &manifests {
            collect_output_paths(config, &mut paths);
        }

        // delete selected paths
        let mut removed = Vec::new();
        let mut errors = Vec::new();
        for path in paths {
            if self.common.dry_run {
                removed.push(path.display().to_string());
                continue;
            }

            match file_system.remove_path(&path) {
                Ok(true) => removed.push(path.display().to_string()),
                Ok(false) => {}
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }

        let exit_code = if errors.is_empty() { 0 } else { 1 };
        let payload = CleanPayload {
            removed,
            errors,
            dry_run: self.common.dry_run,
        };

        Ok(
            CommandOutcome::new(DiagnosticCollection::default(), exit_code, 0, 0, 0)
                .with_data(payload),
        )
    }
}

/// Collect output paths for one config.
fn collect_output_paths(config: &ManifestFile, paths: &mut BTreeSet<PathBuf>) {
    // collect the legacy compiler output directory
    if let Some(out_dir) = config.compiler.out_dir.as_ref() {
        paths.insert(resolve_path(out_dir, &config.directory));
    }

    // collect each target output path
    for target in config.targets.values() {
        let out_dir = resolve_path(&target.destination.directory, &config.directory);
        paths.insert(out_dir);

        if let Some(out_file) = target.destination.file.as_ref() {
            paths.insert(resolve_path(out_file, &config.directory));
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
