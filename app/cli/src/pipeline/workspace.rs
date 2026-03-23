use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_workspace::{Destack, Program, Session, Workspace};

use crate::common::ProgramArgs;
use crate::error::{CliError, CliResult};

/// Resolved workspace context for CLI commands.
#[derive(Debug)]
pub struct WorkspaceContext {
    /// Active session.
    pub session: Arc<Session>,
    /// Active program.
    pub program: Arc<Program>,
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
    let session = args.setup();
    let program = session
        .programs()
        .into_iter()
        .next()
        .ok_or_else(|| CliError::message("session did not create a program"))?;
    let workspace_config = session.workspace_config();
    let resolver = Resolver::from_session(
        &session,
        ResolveOptions::default_for_workspace(session.cwd.clone(), workspace_config.as_deref()),
    );
    let workspace = Arc::new(session.workspace_snapshot());

    Ok(WorkspaceContext {
        session,
        program,
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

/// Load and parse a destack.json file.
pub fn load_destack_config(resolver: &Resolver, path: &Path) -> CliResult<Destack> {
    // map resolver errors into strings
    resolver
        .read_destack_config(path, CachePolicy::UseCache)
        .map_err(|error| CliError::message(error.to_string()))
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
pub fn load_destack_config_for_program(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<Destack> {
    let path = resolve_destack_config_path(program_args, resolver, cwd)?;
    load_destack_config(resolver, &path)
}

/// Resolve the default target from destack.json when available.
pub fn default_target_for_program(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<Option<String>> {
    // honor explicit config paths
    if program_args.config.is_some() {
        let config = load_destack_config_for_program(program_args, resolver, cwd)?;
        return Ok(config.options.default_target);
    }

    // fall back to auto discovery when present
    let Some(path) = find_destack_config(resolver, cwd) else {
        return Ok(None);
    };
    let config = load_destack_config(resolver, &path)?;
    Ok(config.options.default_target)
}

/// Resolve the default target using a session resolver.
pub fn default_target_for_session(
    program_args: &ProgramArgs,
    session: &Session,
) -> CliResult<Option<String>> {
    // build a resolver using the session configuration
    let workspace_config = session.workspace_config();
    let resolver = Resolver::from_session(
        session,
        ResolveOptions::default_for_workspace(session.cwd.clone(), workspace_config.as_deref()),
    );

    default_target_for_program(program_args, &resolver, &session.cwd)
}

/// Resolve a destack.json for a path and load it.
pub fn load_destack_config_for_path(resolver: &Resolver, path: &Path) -> CliResult<Destack> {
    // resolve the config path from the directory
    let destack_config_path = find_destack_config(resolver, path)
        .ok_or_else(|| CliError::message("destack.json not found"))?;

    // load the resolved config
    load_destack_config(resolver, &destack_config_path)
}

/// Load destack.json files for all packages in a workspace.
pub fn load_workspace_configs(
    resolver: &Resolver,
    workspace: &Workspace,
) -> CliResult<Vec<Destack>> {
    // collect unique config paths
    let mut configs = BTreeMap::new();

    // collect config paths for each package path
    for package_path in &workspace.package_paths {
        if let Some(path) = find_destack_config(resolver, package_path) {
            configs.entry(path).or_insert_with(|| package_path.clone());
        }
    }

    // load unique configs in a stable order
    let mut resolved = Vec::new();
    for (path, _) in configs {
        resolved.push(load_destack_config(resolver, &path)?);
    }

    // return the loaded configs
    Ok(resolved)
}
