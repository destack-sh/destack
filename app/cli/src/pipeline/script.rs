use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::common::ProgramArgs;
use crate::error::{CliError, CliResult};
use crate::pipeline::workspace::{
    find_destack_config, resolve_destack_config_path, workspace_context,
};

/// Source of a resolved script command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSource {
    /// Script came from destack.json tasks.
    Destack,
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

/// Task specification loaded from destack.json.
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

/// Resolve a script command from destack.json tasks.
pub fn resolve_script_command(
    program_args: &ProgramArgs,
    script_name: &str,
) -> CliResult<Option<ScriptCommand>> {
    let context = workspace_context(program_args, None)?;
    let cwd = program_args.effective_cwd();
    resolve_script_command_for_repository(
        program_args,
        script_name,
        &context.repository,
        context.revision,
        &cwd,
    )
}

/// Load task specifications from a destack.json file.
pub fn load_tasks(
    repository: &destack_workspace::Repository,
    revision: destack_workspace::Revision,
    destack_config_path: &Path,
) -> CliResult<Vec<TaskSpec>> {
    // read the config file from repository truth
    let file_id = repository.file_id(destack_config_path);
    let file = repository
        .file(revision, file_id)
        .map_err(|error| {
            CliError::message(format!(
                "failed to load {}: {error}",
                destack_config_path.display()
            ))
        })?
        .ok_or_else(|| {
            CliError::message(format!("failed to load {}", destack_config_path.display()))
        })?;
    let content = file.text();

    // parse the config json
    let value: Value = serde_json::from_str(content)
        .map_err(|error| CliError::message(format!("invalid destack.json: {error}")))?;

    // extract the task map
    let Some(tasks_value) = value.get("tasks") else {
        return Ok(Vec::new());
    };
    let tasks_object = tasks_value
        .as_object()
        .ok_or_else(|| CliError::message("tasks must be an object"))?;

    // resolve the config directory
    let config_dir = destack_config_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| destack_config_path.to_path_buf());

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
                config_dir.join(path)
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

/// Resolve a destack.json task by name.
fn resolve_destack_config_task(
    program_args: &ProgramArgs,
    name: &str,
    repository: &destack_workspace::Repository,
    revision: destack_workspace::Revision,
    cwd: &Path,
) -> CliResult<Option<TaskSpec>> {
    let destack_config_path = if program_args.config.is_some() {
        Some(resolve_destack_config_path(
            program_args,
            repository,
            revision,
            cwd,
        )?)
    } else {
        find_destack_config(repository, revision, cwd)
    };

    let Some(destack_config_path) = destack_config_path else {
        return Ok(None);
    };

    let tasks = load_tasks(repository, revision, &destack_config_path)?;
    Ok(tasks.into_iter().find(|task| task.name == name))
}

/// Resolve a script command from one repository revision.
pub(crate) fn resolve_script_command_for_repository(
    program_args: &ProgramArgs,
    script_name: &str,
    repository: &destack_workspace::Repository,
    revision: destack_workspace::Revision,
    cwd: &Path,
) -> CliResult<Option<ScriptCommand>> {
    if let Some(task) =
        resolve_destack_config_task(program_args, script_name, repository, revision, cwd)?
    {
        let cwd = task
            .cwd
            .unwrap_or_else(|| task_base_dir(program_args, repository, revision, cwd));
        return Ok(Some(ScriptCommand {
            name: task.name,
            command: task.command,
            cwd,
            source: ScriptSource::Destack,
        }));
    }

    Ok(None)
}

/// Resolve the base directory for destack.json tasks.
fn task_base_dir(
    program_args: &ProgramArgs,
    repository: &destack_workspace::Repository,
    revision: destack_workspace::Revision,
    cwd: &Path,
) -> PathBuf {
    // resolve base dir for tasks when no cwd override is provided
    if let Some(path) = find_destack_config(repository, revision, cwd)
        && let Some(parent) = path.parent()
    {
        return parent.to_path_buf();
    }

    if let Some(config) = program_args.config.as_ref() {
        let config_path = if config.is_absolute() {
            config.clone()
        } else {
            cwd.join(config)
        };

        if let Ok(Some(metadata)) = repository.file_metadata(revision, &config_path) {
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
