use std::path::{Path, PathBuf};
use std::process::Command;

use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Task entry for task list output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTaskEntry {
    /// Task name.
    pub name: String,
    /// Task description.
    pub description: Option<String>,
}

/// Payload for task command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTaskPayload {
    /// Task list entries.
    pub tasks: Option<Vec<CommandTaskEntry>>,
    /// Task name.
    pub task: Option<String>,
    /// Task command string.
    pub command: Option<String>,
    /// Task working directory.
    pub cwd: Option<String>,
    /// Whether this was a dry run.
    pub dry_run: Option<bool>,
    /// Exit code when executed.
    pub exit_code: Option<i32>,
}

/// Task selection for the task command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandTaskAction {
    /// List available tasks.
    List,
    /// Run a task by name.
    Run {
        /// Task name.
        name: String,
        /// Task arguments.
        args: Vec<String>,
        /// Whether to only print the command.
        dry_run: bool,
    },
}

/// Options for the task command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandTaskOptions {
    /// Task action to run.
    pub action: CommandTaskAction,
}

impl CommandContext<'_> {
    /// Execute a task command.
    pub(super) fn run_task_command(
        &mut self,
        options: &CommandTaskOptions,
    ) -> Result<CommandOutcome, String> {
        // resolve dsconfig path
        let dsconfig_path = self.resolve_dsconfig_path(self.common.config_path.as_deref())?;

        // load task definitions
        let resolver = self.resolver();
        let tasks = load_tasks(&resolver, &dsconfig_path)?;
        let dsconfig_dir = dsconfig_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| dsconfig_path.clone());

        match &options.action {
            CommandTaskAction::List => {
                let entries: Vec<CommandTaskEntry> = tasks
                    .into_iter()
                    .map(|task| CommandTaskEntry {
                        name: task.name,
                        description: task.description,
                    })
                    .collect();
                let payload = CommandTaskPayload {
                    tasks: Some(entries),
                    task: None,
                    command: None,
                    cwd: None,
                    dry_run: None,
                    exit_code: None,
                };
                let data = serde_json::to_value(payload)
                    .map_err(|error| format!("invalid task payload: {error}"))?;
                Ok(
                    CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0, None)
                        .with_data(data),
                )
            }
            CommandTaskAction::Run {
                name,
                args,
                dry_run,
            } => {
                let task = tasks
                    .iter()
                    .find(|task| task.name == *name)
                    .ok_or_else(|| format!("task '{name}' not found"))?;

                let mut command = task.command.clone();
                if !args.is_empty() {
                    command.push(' ');
                    command.push_str(&args.join(" "));
                }

                let cwd = task.cwd.clone().unwrap_or_else(|| dsconfig_dir.clone());

                if *dry_run {
                    let payload = CommandTaskPayload {
                        tasks: None,
                        task: Some(task.name.clone()),
                        command: Some(command),
                        cwd: Some(cwd.display().to_string()),
                        dry_run: Some(true),
                        exit_code: None,
                    };
                    let data = serde_json::to_value(payload)
                        .map_err(|error| format!("invalid task payload: {error}"))?;
                    return Ok(CommandOutcome::new(
                        DiagnosticCollection::default(),
                        0,
                        0,
                        0,
                        0,
                        None,
                    )
                    .with_data(data));
                }

                let mut shell = shell_command(&command);
                shell.current_dir(&cwd);
                let output_result = shell
                    .output()
                    .map_err(|error| format!("task failed: {error}"))?;

                if !output_result.stdout.is_empty() {
                    self.output.push_stdout(output_result.stdout);
                }
                if !output_result.stderr.is_empty() {
                    self.output.push_stderr(output_result.stderr);
                }

                let exit_code = output_result.status.code().unwrap_or(1);
                let payload = CommandTaskPayload {
                    tasks: None,
                    task: Some(task.name.clone()),
                    command: Some(command),
                    cwd: Some(cwd.display().to_string()),
                    dry_run: Some(false),
                    exit_code: Some(exit_code),
                };
                let data = serde_json::to_value(payload)
                    .map_err(|error| format!("invalid task payload: {error}"))?;
                Ok(
                    CommandOutcome::new(DiagnosticCollection::default(), exit_code, 0, 0, 0, None)
                        .with_data(data),
                )
            }
        }
    }
}

/// Task specification loaded from dsconfig.json.
#[derive(Debug, Clone)]
struct TaskSpec {
    /// The task name.
    name: String,
    /// The shell command to execute.
    command: String,
    /// The task description.
    description: Option<String>,
    /// The working directory for the task.
    cwd: Option<PathBuf>,
}

/// Load task specifications from a dsconfig.json file.
fn load_tasks(
    resolver: &destack_resolver::Resolver,
    dsconfig_path: &Path,
) -> Result<Vec<TaskSpec>, String> {
    let content = resolver
        .fs
        .read_to_string(dsconfig_path)
        .map_err(|error| format!("failed to read {}: {error}", dsconfig_path.display()))?;

    let value: Value =
        serde_json::from_str(&content).map_err(|error| format!("invalid dsconfig: {error}"))?;

    let Some(tasks_value) = value.get("tasks") else {
        return Ok(Vec::new());
    };
    let tasks_object = tasks_value
        .as_object()
        .ok_or_else(|| "tasks must be an object".to_string())?;

    let dsconfig_dir = dsconfig_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| dsconfig_path.to_path_buf());

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

        let Some(command) = value.get("command").and_then(|v| v.as_str()) else {
            return Err(format!("task '{name}' is missing a command"));
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
fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        return cmd;
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}
