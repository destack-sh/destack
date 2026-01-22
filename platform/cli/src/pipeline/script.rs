use std::path::{Path, PathBuf};
use std::process::Command;

use destack_workspace::PackageJson;
use serde_json::Value;

use crate::common::ProgramArgs;
use crate::error::{CliError, CliResult};
use crate::pipeline::workspace::{
    find_dsconfig, load_dsconfig, resolve_dsconfig_path, workspace_context,
};

/// Source of a resolved script command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSource {
    /// Script came from dsconfig.json tasks.
    DsConfig,
    /// Script came from package.json scripts.
    PackageJson,
}

/// Resolved script command for execution.
#[derive(Debug, Clone)]
pub struct ScriptCommand {
    /// Script name.
    pub name: String,
    /// Shell command to execute.
    pub command: String,
    /// Working directory for execution.
    pub cwd: PathBuf,
    /// Origin of the script command.
    pub source: ScriptSource,
}

/// Task specification loaded from dsconfig.json.
#[derive(Debug, Clone)]
pub struct TaskSpec {
    /// The task name.
    pub name: String,
    /// The shell command to execute.
    pub command: String,
    /// The task description.
    pub description: Option<String>,
    /// The working directory for the task.
    pub cwd: Option<PathBuf>,
}

/// Resolve a script command from dsconfig tasks or package.json scripts.
pub fn resolve_script_command(
    program_args: &ProgramArgs,
    script_name: &str,
) -> CliResult<Option<ScriptCommand>> {
    let context = workspace_context(program_args, None)?;
    let cwd = context.session.cwd.clone();
    resolve_script_command_with_resolver(program_args, script_name, &context.resolver, &cwd)
}

/// Load task specifications from a dsconfig.json file.
pub fn load_tasks(
    resolver: &destack_resolver::Resolver,
    dsconfig_path: &Path,
) -> CliResult<Vec<TaskSpec>> {
    // read the dsconfig file
    let content = resolver.fs.read_to_string(dsconfig_path).map_err(|error| {
        CliError::message(format!(
            "failed to read {}: {error}",
            dsconfig_path.display()
        ))
    })?;

    // parse the config json
    let value: Value = serde_json::from_str(&content)
        .map_err(|error| CliError::message(format!("invalid dsconfig: {error}")))?;

    // extract the task map
    let Some(tasks_value) = value.get("tasks") else {
        return Ok(Vec::new());
    };
    let tasks_object = tasks_value
        .as_object()
        .ok_or_else(|| CliError::message("tasks must be an object"))?;

    // resolve the dsconfig directory
    let dsconfig_dir = dsconfig_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| dsconfig_path.to_path_buf());

    // build task specs from json values
    let mut tasks = Vec::new();
    for (name, value) in tasks_object {
        if let Some(command) = value.as_str() {
            tasks.push(TaskSpec {
                name: name.clone(),
                command: command.to_string(),
                description: None,
                cwd: None,
            });
            continue;
        }

        // parse object form of the task
        let Some(command) = value.get("command").and_then(|v| v.as_str()) else {
            return Err(CliError::message(format!(
                "task '{name}' is missing a command"
            )));
        };
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);
        let cwd = value.get("cwd").and_then(|v| v.as_str()).map(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                dsconfig_dir.join(path)
            }
        });

        // push the expanded task spec
        tasks.push(TaskSpec {
            name: name.clone(),
            command: command.to_string(),
            description,
            cwd,
        });
    }

    Ok(tasks)
}

/// Build a shell command for script execution.
pub fn shell_command(command: &str) -> Command {
    // use cmd on windows
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        return cmd;
    }

    // default to sh on unix
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

/// Resolve a dsconfig task by name.
fn resolve_dsconfig_task(
    program_args: &ProgramArgs,
    name: &str,
    resolver: &destack_resolver::Resolver,
    cwd: &Path,
) -> CliResult<Option<TaskSpec>> {
    let dsconfig_path = if program_args.config.is_some() {
        Some(resolve_dsconfig_path(program_args, resolver, cwd)?)
    } else {
        find_dsconfig(resolver, cwd)
    };

    let Some(dsconfig_path) = dsconfig_path else {
        return Ok(None);
    };

    let tasks = load_tasks(resolver, &dsconfig_path)?;
    Ok(tasks.into_iter().find(|task| task.name == name))
}

/// Resolve a script command using the provided resolver and cwd.
pub(crate) fn resolve_script_command_with_resolver(
    program_args: &ProgramArgs,
    script_name: &str,
    resolver: &destack_resolver::Resolver,
    cwd: &Path,
) -> CliResult<Option<ScriptCommand>> {
    if let Some(task) = resolve_dsconfig_task(program_args, script_name, resolver, cwd)? {
        let cwd = task
            .cwd
            .unwrap_or_else(|| task_base_dir(program_args, resolver, cwd));
        return Ok(Some(ScriptCommand {
            name: task.name,
            command: task.command,
            cwd,
            source: ScriptSource::DsConfig,
        }));
    }

    if let Some(script) = resolve_package_script(script_name, resolver, cwd)? {
        return Ok(Some(script));
    }

    Ok(None)
}

/// Resolve a package.json script by name.
fn resolve_package_script(
    name: &str,
    resolver: &destack_resolver::Resolver,
    cwd: &Path,
) -> CliResult<Option<ScriptCommand>> {
    let Some(package_path) = find_package_json(resolver, cwd)? else {
        return Ok(None);
    };

    let content = resolver.fs.read_to_string(&package_path).map_err(|error| {
        CliError::message(format!(
            "failed to read {}: {error}",
            package_path.display()
        ))
    })?;
    let package: PackageJson = serde_json::from_str(&content)
        .map_err(|error| CliError::message(format!("invalid package.json: {error}")))?;
    let Some(scripts) = package.scripts else {
        return Ok(None);
    };
    let Some(command) = scripts.get(name) else {
        return Ok(None);
    };

    let cwd = package_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.to_path_buf());

    Ok(Some(ScriptCommand {
        name: name.to_string(),
        command: command.to_string(),
        cwd,
        source: ScriptSource::PackageJson,
    }))
}

/// Find the nearest package.json path by walking up directories.
fn find_package_json(
    resolver: &destack_resolver::Resolver,
    cwd: &Path,
) -> CliResult<Option<PathBuf>> {
    let mut current = cwd;
    loop {
        let candidate = current.join("package.json");
        if let Ok(metadata) = resolver.fs.metadata(&candidate)
            && metadata.is_file
        {
            return Ok(Some(candidate));
        }

        let Some(parent) = current.parent() else {
            return Ok(None);
        };
        if parent == current {
            return Ok(None);
        }
        current = parent;
    }
}

/// Resolve the base directory for dsconfig tasks.
fn task_base_dir(
    program_args: &ProgramArgs,
    resolver: &destack_resolver::Resolver,
    cwd: &Path,
) -> PathBuf {
    // resolve base dir for tasks when no cwd override is provided
    if let Some(path) = find_dsconfig(resolver, cwd)
        && let Ok(dsconfig) = load_dsconfig(resolver, &path)
    {
        return dsconfig.directory;
    }

    if let Some(config) = program_args.config.as_ref() {
        let config_path = if config.is_absolute() {
            config.clone()
        } else {
            cwd.join(config)
        };

        if let Ok(metadata) = resolver.fs.metadata(&config_path) {
            if metadata.is_directory {
                return config_path;
            }
            if metadata.is_file
                && let Some(parent) = config_path.parent()
            {
                return parent.to_path_buf();
            }
        }
    }

    cwd.to_path_buf()
}
