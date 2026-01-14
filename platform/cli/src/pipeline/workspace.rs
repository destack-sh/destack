use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_resolver::{ResolveOptions, Resolver};
use destack_workspace::{DsConfig, Program, Session, Workspace};

use crate::common::ProgramArgs;

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
) -> Result<WorkspaceContext, String> {
    // clone program args and apply overrides
    let mut args = program_args.clone();
    if let Some(cwd) = cwd_override {
        args.cwd = Some(cwd);
    }

    // initialize the session and program
    let session = args.setup();
    let program = session
        .programs
        .iter()
        .next()
        .map(|entry| entry.value().clone())
        .ok_or_else(|| "session did not create a program".to_string())?;

    // create a resolver for workspace queries
    let resolver = Resolver::new(
        session.fs.clone(),
        session.files.clone(),
        session.packages.clone(),
        session.tsconfigs.clone(),
        ResolveOptions::default(),
    );

    // capture the workspace handle
    let workspace = session.workspace.clone();

    Ok(WorkspaceContext {
        session,
        program,
        resolver,
        workspace,
    })
}

/// Locate a dsconfig.json path for a directory.
pub fn find_dsconfig(resolver: &Resolver, cwd: &Path) -> Option<PathBuf> {
    // delegate to the resolver lookup
    resolver.find_dsconfig(cwd)
}

/// Load and parse a dsconfig.json file.
pub fn load_dsconfig(resolver: &Resolver, path: &Path) -> Result<DsConfig, String> {
    // map resolver errors into strings
    resolver.load_dsconfig(path).map_err(|e| e.to_string())
}

/// Resolve a dsconfig.json path based on program args.
pub fn resolve_dsconfig_path(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> Result<PathBuf, String> {
    // honor explicit config paths when provided
    if let Some(config) = program_args.config.as_ref() {
        let path = if config.is_absolute() {
            config.clone()
        } else {
            cwd.join(config)
        };

        if path.is_dir() {
            return find_dsconfig(resolver, &path)
                .ok_or_else(|| "dsconfig.json not found".to_string());
        }

        if path.is_file() {
            return Ok(path);
        }

        return Err(format!("config path not found: {}", path.display()));
    }

    // fall back to cwd lookup
    find_dsconfig(resolver, cwd).ok_or_else(|| "dsconfig.json not found".to_string())
}

/// Resolve a dsconfig.json path and load it.
pub fn load_dsconfig_for_program(
    program_args: &ProgramArgs,
    resolver: &Resolver,
    cwd: &Path,
) -> Result<DsConfig, String> {
    let path = resolve_dsconfig_path(program_args, resolver, cwd)?;
    load_dsconfig(resolver, &path)
}

/// Resolve a dsconfig.json for a path and load it.
pub fn load_dsconfig_for_path(resolver: &Resolver, path: &Path) -> Result<DsConfig, String> {
    // resolve the config path from the directory
    let dsconfig_path =
        find_dsconfig(resolver, path).ok_or_else(|| "dsconfig.json not found".to_string())?;

    // load the resolved config
    load_dsconfig(resolver, &dsconfig_path)
}

/// Load dsconfig.json files for all packages in a workspace.
pub fn load_workspace_dsconfigs(
    resolver: &Resolver,
    workspace: &Workspace,
) -> Result<Vec<DsConfig>, String> {
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
