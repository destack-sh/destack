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
        &repository,
        ResolveOptions::default_for_workspace(repository.cwd.clone(), workspace_options.as_ref()),
    );
    Ok(WorkspaceContext {
        repository,
        resolver,
        workspace,
    })
}

/// Locate a destack.json path for a directory.
pub fn find_destack_config(resolver: &Resolver, cwd: &Path) -> Option<PathBuf> {
    // walk up directories looking for destack.json
    let mut current = cwd.to_path_buf();
    loop {
        let candidate = current.join("destack.json");
        if resolver
            .fs
            .metadata(&candidate)
            .is_ok_and(|metadata| metadata.is_file)
        {
            return Some(candidate);
        }

        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }

    None
}

/// Load one `destack.json` declaration from disk.
pub fn load_destack_declaration(
    repository: &Repository,
    _resolver: &Resolver,
    path: &Path,
) -> CliResult<DestackDeclaration> {
    repository
        .load_destack_declaration_for_path(path)
        .ok_or_else(|| CliError::message(format!("failed to load {}", path.display())))
}

/// Resolve a destack.json path based on program args.
pub fn resolve_destack_config_path(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<PathBuf> {
    // honor explicit config paths when provided
    if let Some(config) = program_args.config.as_ref() {
        let path = if config.is_absolute() {
            config.clone()
        } else {
            cwd.join(config)
        };

        let metadata = resolver
            .fs
            .metadata(&path)
            .map_err(|_| CliError::message(format!("config path not found: {}", path.display())))?;

        if metadata.is_directory {
            return find_destack_config(resolver, &path)
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
    find_destack_config(resolver, cwd).ok_or_else(|| CliError::message("destack.json not found"))
}

/// Resolve a destack.json path and load it.
pub fn load_destack_declaration_for_program(
    program_args: &ProgramArgs,
    repository: &Repository,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<DestackDeclaration> {
    let path = resolve_destack_config_path(program_args, resolver, cwd)?;
    load_destack_declaration(repository, resolver, &path)
}

/// Resolve the default target from destack.json when available.
pub fn default_target_for_program(
    program_args: &ProgramArgs,
    repository: &Repository,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<Option<String>> {
    // honor explicit config paths
    if program_args.config.is_some() {
        let declaration =
            load_destack_declaration_for_program(program_args, repository, resolver, cwd)?;
        return Ok(declaration.package_options().default_target);
    }

    // fall back to auto discovery when present
    let Some(path) = find_destack_config(resolver, cwd) else {
        return Ok(None);
    };
    let declaration = load_destack_declaration(repository, resolver, &path)?;
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
    let workspace_options = repository.workspace_options(revision).map_err(|error| {
        CliError::message(format!("failed to derive workspace options: {error}"))
    })?;
    let resolver = Resolver::from_repository(
        repository,
        ResolveOptions::default_for_workspace(repository.cwd.clone(), workspace_options.as_ref()),
    );

    default_target_for_program(program_args, repository, &resolver, &repository.cwd)
}

/// Resolve a destack.json for a path and load it.
pub fn load_destack_declaration_for_path(
    repository: &Repository,
    resolver: &Resolver,
    path: &Path,
) -> CliResult<DestackDeclaration> {
    // resolve the config path from the directory
    let destack_config_path = find_destack_config(resolver, path)
        .ok_or_else(|| CliError::message("destack.json not found"))?;

    // load the resolved config
    load_destack_declaration(repository, resolver, &destack_config_path)
}

/// Load destack.json files for all packages in a workspace.
pub fn load_workspace_declarations(
    repository: &Repository,
    resolver: &Resolver,
    revision: Revision,
) -> CliResult<Vec<DestackDeclaration>> {
    // collect unique config paths
    let mut configs = BTreeMap::new();
    let package_paths = repository
        .workspace_package_paths(revision)
        .map_err(|error| CliError::message(format!("failed to derive package paths: {error}")))?;

    // collect config paths for each package path
    for package_path in &package_paths {
        if let Some(path) = find_destack_config(resolver, package_path) {
            configs.entry(path).or_insert_with(|| package_path.clone());
        }
    }

    // load unique configs in a stable order
    let mut resolved = Vec::new();
    for (path, _) in configs {
        resolved.push(load_destack_declaration(repository, resolver, &path)?);
    }

    // return the loaded configs
    Ok(resolved)
}
