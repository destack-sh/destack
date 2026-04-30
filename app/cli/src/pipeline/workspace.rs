use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_resolver::{ResolveOptions, Resolver};
use destack_workspace::{DestackDeclaration, Repository, Revision, Workspace};

use crate::common::ProgramArgs;
use crate::error::{CliError, CliResult};

/// Resolved workspace context for CLI commands.
#[derive(Debug)]
pub struct WorkspaceContext {
    /// Active repository.
    pub repository: Arc<Repository>,
    /// Active repository revision.
    pub revision: Revision,
    /// Resolver for workspace lookups.
    pub resolver: Resolver,
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
    let workspace_options = repository.workspace_options(revision).map_err(|error| {
        CliError::message(format!("failed to derive workspace options: {error}"))
    })?;
    let resolver = Resolver::from_repository(
        repository.clone(),
        ResolveOptions::default_for_workspace(
            repository.workspace_root().to_path_buf(),
            workspace_options.as_ref(),
        ),
    );
    Ok(WorkspaceContext {
        repository,
        revision,
        resolver,
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
        match repository.destack_declaration_for_path(revision, &candidate) {
            Ok(Some(_)) => return Some(candidate),
            Ok(None) => {}
            Err(_) => return None,
        }

        if !directory.pop() {
            return None;
        }
    }
}

/// Load one `destack.json` declaration from one revision.
pub fn load_destack_declaration(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> CliResult<DestackDeclaration> {
    repository
        .destack_declaration_for_path(revision, path)
        .map_err(|error| CliError::message(format!("failed to load {}: {error}", path.display())))?
        .map(|declaration| declaration.as_ref().clone())
        .ok_or_else(|| CliError::message(format!("failed to load {}", path.display())))
}

/// Resolve a destack.json path based on program args.
pub fn resolve_destack_config_path(
    program_args: &ProgramArgs,
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> CliResult<PathBuf> {
    // honor explicit config paths when provided
    if let Some(config) = program_args.config.as_ref() {
        let path = if config.is_absolute() {
            config.clone()
        } else {
            cwd.join(config)
        };

        let metadata = repository
            .file_metadata(revision, &path)
            .map_err(|error| {
                CliError::message(format!(
                    "failed to read config path {}: {error}",
                    path.display()
                ))
            })?
            .ok_or_else(|| {
                CliError::message(format!("config path not found: {}", path.display()))
            })?;

        if metadata.is_directory {
            return find_destack_config(repository, revision, &path)
                .ok_or_else(|| CliError::message("destack.json not found"));
        }

        if metadata.is_file {
            return Ok(path);
        }

        return Err(CliError::message(format!(
            "config path not found: {}",
            path.display()
        )));
    }

    // fall back to cwd lookup
    find_destack_config(repository, revision, cwd)
        .ok_or_else(|| CliError::message("destack.json not found"))
}

/// Resolve a destack.json path and load it.
pub fn load_destack_declaration_for_program(
    program_args: &ProgramArgs,
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> CliResult<DestackDeclaration> {
    let path = resolve_destack_config_path(program_args, repository, revision, cwd)?;
    load_destack_declaration(repository, revision, &path)
}

/// Resolve the default target from destack.json when available.
pub fn default_target_for_program(
    program_args: &ProgramArgs,
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
) -> CliResult<Option<String>> {
    // honor explicit config paths
    if program_args.config.is_some() {
        let declaration =
            load_destack_declaration_for_program(program_args, repository, revision, cwd)?;
        return Ok(declaration.package_options().default_target);
    }

    // fall back to auto discovery when present
    let Some(path) = find_destack_config(repository, revision, cwd) else {
        return Ok(None);
    };
    let declaration = load_destack_declaration(repository, revision, &path)?;
    Ok(declaration.package_options().default_target)
}

/// Resolve the default target using a repository resolver.
pub fn default_target_for_repository(
    program_args: &ProgramArgs,
    repository: &Repository,
) -> CliResult<Option<String>> {
    // build a resolver using the repository configuration
    let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
    let revision = repository.current(&reference).map_err(|error| {
        CliError::message(format!("failed to resolve current revision: {error}"))
    })?;
    default_target_for_program(
        program_args,
        repository,
        revision,
        repository.workspace_root(),
    )
}

/// Resolve a destack.json for a path and load it.
pub fn load_destack_declaration_for_path(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> CliResult<DestackDeclaration> {
    // resolve the config path from the directory
    let destack_config_path = find_destack_config(repository, revision, path)
        .ok_or_else(|| CliError::message("destack.json not found"))?;

    // load the resolved config
    load_destack_declaration(repository, revision, &destack_config_path)
}

/// Load destack.json files for all packages in a workspace.
pub fn load_workspace_declarations(
    repository: &Repository,
    revision: Revision,
) -> CliResult<Vec<DestackDeclaration>> {
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
        resolved.push(load_destack_declaration(repository, revision, &path)?);
    }

    // return the loaded configs
    Ok(resolved)
}
