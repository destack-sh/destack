use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_workspace::{DestackFile, Repository, Revision, Workspace};

use crate::common::ProgramArgs;
use crate::error::{CliError, CliResult};

/// Resolved workspace context for CLI commands.
#[derive(Debug)]
pub struct WorkspaceContext {
    /// Active repository.
    pub repository: Arc<Repository>,
    /// Active repository revision.
    pub revision: Revision,
    /// Discovered workspace.
    pub workspace: Arc<Workspace>,
}

/// Build a workspace context for CLI commands.
pub fn workspace_context(
    program_args: &ProgramArgs,
    cwd_override: Option<PathBuf>,
) -> CliResult<WorkspaceContext> {
    // clone program args and apply overrides
    let mut args = program_args.clone();
    if let Some(cwd) = cwd_override {
        args.cwd = Some(cwd);
    }

    // initialize
    let repository = args.setup();
    let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
    let revision = repository.current(&reference).map_err(|error| {
        CliError::message(format!("failed to resolve current revision: {error}"))
    })?;
    let workspace = repository
        .workspace(revision)
        .map_err(|error| CliError::message(format!("failed to derive workspace: {error}")))?;
    Ok(WorkspaceContext {
        repository,
        revision,
        workspace,
    })
}

/// Locate a destack.json path for a directory.
pub fn find_destack_config(
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> Option<PathBuf> {
    let mut directory = if repository
        .file_metadata(revision, cwd)
        .ok()
        .flatten()
        .is_some_and(|metadata| metadata.is_directory)
    {
        cwd.to_path_buf()
    } else {
        cwd.parent()?.to_path_buf()
    };

    loop {
        let candidate = directory.join("destack.json");
        if repository
            .file_metadata(revision, &candidate)
            .ok()
            .flatten()
            .is_some_and(|metadata| metadata.is_file)
        {
            return Some(candidate);
        }

        if !directory.pop() {
            return None;
        }
    }
}

/// Load one `destack.json` config from one revision.
pub fn load_destack_config(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> CliResult<DestackFile> {
    repository
        .inherited_destack_for_path(revision, path)
        .map_err(|error| CliError::message(format!("failed to load {}: {error}", path.display())))?
        .ok_or_else(|| CliError::message(format!("destack.json not found: {}", path.display())))
}

/// Resolve a destack.json path based on program args.
pub fn resolve_destack_config_path(
    program_args: &ProgramArgs,
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> CliResult<PathBuf> {
    // honor explicit manifest paths when provided
    if let Some(manifest) = program_args.manifest.as_ref() {
        let path = if manifest.is_absolute() {
            manifest.clone()
        } else {
            cwd.join(manifest)
        };

        let metadata = repository
            .file_metadata(revision, &path)
            .map_err(|error| {
                CliError::message(format!(
                    "failed to read manifest path {}: {error}",
                    path.display()
                ))
            })?
            .ok_or_else(|| {
                CliError::message(format!("manifest path not found: {}", path.display()))
            })?;

        if metadata.is_directory {
            return find_destack_config(repository, revision, &path)
                .ok_or_else(|| CliError::message("destack.json not found"));
        }

        if metadata.is_file {
            return Ok(path);
        }

        return Err(CliError::message(format!(
            "manifest path not found: {}",
            path.display()
        )));
    }

    // fall back to cwd lookup
    find_destack_config(repository, revision, cwd)
        .ok_or_else(|| CliError::message("destack.json not found"))
}

/// Resolve a destack.json path and load it.
pub fn load_destack_config_for_program(
    program_args: &ProgramArgs,
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> CliResult<DestackFile> {
    let path = resolve_destack_config_path(program_args, repository, revision, cwd)?;
    load_destack_config(repository, revision, &path)
}

/// Resolve the default target from destack.json when available.
pub fn default_target_for_program(
    program_args: &ProgramArgs,
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> CliResult<Option<String>> {
    // honor explicit manifest paths
    if program_args.manifest.is_some() {
        let config = load_destack_config_for_program(program_args, repository, revision, cwd)?;
        return Ok(config.default_target.clone());
    }

    // fall back to auto discovery when present
    let Some(path) = find_destack_config(repository, revision, cwd) else {
        return Ok(None);
    };
    let config = load_destack_config(repository, revision, &path)?;
    Ok(config.default_target.clone())
}

/// Resolve the default target from repository config.
pub fn default_target_for_repository(
    program_args: &ProgramArgs,
    repository: Arc<Repository>,
) -> CliResult<Option<String>> {
    // resolve current repository revision
    let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
    let revision = repository.current(&reference).map_err(|error| {
        CliError::message(format!("failed to resolve current revision: {error}"))
    })?;

    default_target_for_program(
        program_args,
        &repository,
        revision,
        repository.workspace_root(),
    )
}

/// Resolve a destack.json for a path and load it.
pub fn load_destack_config_for_path(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> CliResult<DestackFile> {
    // resolve the config path from the directory
    let destack_config_path = find_destack_config(repository, revision, path)
        .ok_or_else(|| CliError::message("destack.json not found"))?;

    // load the resolved config
    load_destack_config(repository, revision, &destack_config_path)
}

/// Load destack.json files for all packages in a workspace.
pub fn load_workspace_configs(
    repository: &Repository,
    revision: Revision,
) -> CliResult<Vec<DestackFile>> {
    // collect unique config paths
    let mut configs = BTreeMap::new();
    let package_paths = repository
        .package_roots(revision)
        .map_err(|error| CliError::message(format!("failed to derive package paths: {error}")))?;

    // collect config paths for each package path
    for package_path in &package_paths {
        if let Some(path) = find_destack_config(repository, revision, package_path) {
            configs.entry(path).or_insert_with(|| package_path.clone());
        }
    }

    // load unique configs in a stable order
    let mut resolved = Vec::new();
    for (path, _) in configs {
        resolved.push(load_destack_config(repository, revision, &path)?);
    }

    // return the loaded configs
    Ok(resolved)
}
