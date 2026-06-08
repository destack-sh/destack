use std::path::{Path, PathBuf};
use std::process::Command;

use destack_repository::DestackFile;

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
    pub command: Option<String>,
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
    repository: &destack_repository::Repository,
    revision: destack_repository::Revision,
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
    let config = DestackFile::parse(&file)
        .map_err(|error| CliError::message(format!("invalid destack.json: {error}")))?;

    // build task specs from json values
    let mut tasks = Vec::new();
    for (name, task) in &config.tasks {
        tasks.push(TaskSpec {
            name: name.clone(),
            command: task.exec.clone(),
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
    repository: &destack_repository::Repository,
    revision: destack_repository::Revision,
    cwd: &Path,
) -> CliResult<Option<TaskSpec>> {
    let destack_config_path = if program_args.manifest.is_some() {
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
    repository: &destack_repository::Repository,
    revision: destack_repository::Revision,
    cwd: &Path,
) -> CliResult<Option<ScriptCommand>> {
    if let Some(task) =
        resolve_destack_config_task(program_args, script_name, repository, revision, cwd)?
    {
        let command = task
            .command
            .ok_or_else(|| CliError::message(format!("task '{}' is missing exec", task.name)))?;
        let cwd = task_base_dir(program_args, repository, revision, cwd);
        return Ok(Some(ScriptCommand {
            name: task.name,
            command,
            cwd,
            source: ScriptSource::Destack,
        }));
    }

    Ok(None)
}

/// Resolve the base directory for destack.json tasks.
fn task_base_dir(
    program_args: &ProgramArgs,
    repository: &destack_repository::Repository,
    revision: destack_repository::Revision,
    cwd: &Path,
) -> PathBuf {
    // resolve base dir for tasks when no cwd override is provided
    if let Some(path) = find_destack_config(repository, revision, cwd)
        && let Some(parent) = path.parent()
    {
        return parent.to_path_buf();
    }

    if let Some(manifest) = program_args.manifest.as_ref() {
        let manifest_path = if manifest.is_absolute() {
            manifest.clone()
        } else {
            cwd.join(manifest)
        };

        if let Ok(Some(metadata)) = repository.file_metadata(revision, &manifest_path) {
            if metadata.is_directory {
                return manifest_path;
            }
            if metadata.is_file
                && let Some(parent) = manifest_path.parent()
            {
                return parent.to_path_buf();
            }
        }
    }

    cwd.to_path_buf()
}
