use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_workspace::{DsConfig, Program, Session, Workspace};

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
        .programs
        .iter()
        .next()
        .map(|entry| entry.value().clone())
        .ok_or_else(|| CliError::message("session did not create a program"))?;
    let resolver = Resolver::from_session(&session, ResolveOptions::default());
    let workspace = Arc::new(session.workspace_snapshot());

    Ok(WorkspaceContext {
        session,
        program,
        resolver,
        workspace,
    })
}

/// Locate a dsconfig.json path for a directory.
pub fn find_dsconfig(resolver: &Resolver, cwd: &Path) -> Option<PathBuf> {
    // walk up directories looking for dsconfig.json
    let mut current = cwd.to_path_buf();
    loop {
        let candidate = current.join("dsconfig.json");
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

/// Load and parse a dsconfig.json file.
pub fn load_dsconfig(resolver: &Resolver, path: &Path) -> CliResult<DsConfig> {
    // map resolver errors into strings
    resolver
        .load_dsconfig(path, CachePolicy::UseCache)
        .map_err(|error| CliError::message(error.to_string()))
}

/// Resolve a dsconfig.json path based on program args.
pub fn resolve_dsconfig_path(
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
            return find_dsconfig(resolver, &path)
                .ok_or_else(|| CliError::message("dsconfig.json not found"));
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
    find_dsconfig(resolver, cwd).ok_or_else(|| CliError::message("dsconfig.json not found"))
}

/// Resolve a dsconfig.json path and load it.
pub fn load_dsconfig_for_program(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<DsConfig> {
    let path = resolve_dsconfig_path(program_args, resolver, cwd)?;
    load_dsconfig(resolver, &path)
}

/// Resolve the default target from dsconfig when available.
pub fn default_target_for_program(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> CliResult<Option<String>> {
    // honor explicit config paths
    if program_args.config.is_some() {
        let dsconfig = load_dsconfig_for_program(program_args, resolver, cwd)?;
        return Ok(dsconfig.options.default_target);
    }

    // fall back to auto discovery when present
    let Some(path) = find_dsconfig(resolver, cwd) else {
        return Ok(None);
    };
    let dsconfig = load_dsconfig(resolver, &path)?;
    Ok(dsconfig.options.default_target)
}

/// Resolve the default target using a session resolver.
pub fn default_target_for_session(
    program_args: &ProgramArgs,
    session: &Session,
) -> CliResult<Option<String>> {
    // build a resolver using the session configuration
    let resolver = Resolver::from_session(session, ResolveOptions::default());

    default_target_for_program(program_args, &resolver, &session.cwd)
}

/// Resolve a dsconfig.json for a path and load it.
pub fn load_dsconfig_for_path(resolver: &Resolver, path: &Path) -> CliResult<DsConfig> {
    // resolve the config path from the directory
    let dsconfig_path = find_dsconfig(resolver, path)
        .ok_or_else(|| CliError::message("dsconfig.json not found"))?;

    // load the resolved config
    load_dsconfig(resolver, &dsconfig_path)
}

/// Load dsconfig.json files for all packages in a workspace.
pub fn load_workspace_dsconfigs(
    resolver: &Resolver,
    workspace: &Workspace,
) -> CliResult<Vec<DsConfig>> {
    // collect unique config paths
    let mut configs = BTreeMap::new();

    // collect dsconfig paths for each package path
    for package_path in &workspace.package_paths {
        if let Some(path) = find_dsconfig(resolver, package_path) {
            configs.entry(path).or_insert_with(|| package_path.clone());
        }
    }

    // load unique configs in a stable order
    let mut resolved = Vec::new();
    for (path, _) in configs {
        resolved.push(load_dsconfig(resolver, &path)?);
    }

    // return the loaded configs
    Ok(resolved)
}
