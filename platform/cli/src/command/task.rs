use std::path::PathBuf;
use std::process::Command;

use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report, report_error,
};
use crate::console;
use crate::pipeline::workspace::{resolve_dsconfig_path, workspace_context};

/// Supported task subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum TaskCommand {
    /// List available tasks.
    List,
    /// Run a task by name.
    Run {
        /// Task name.
        name: String,
        /// Arguments passed to the task.
        #[arg(last = true, value_name = "ARGS")]
        args: Vec<String>,
        /// Show the task command without executing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Arguments for the task command.
#[derive(Args, Debug, Clone)]
pub struct TaskArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// The task command to run (defaults to list).
    #[command(subcommand)]
    pub command: Option<TaskCommand>,
}

/// Run workspace tasks.
pub fn run(args: &TaskArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("task", &args.program, &args.report) {
        return code;
    }

    // guard against unsupported json output
    if args.report.is_json() && matches!(args.command, Some(TaskCommand::Run { .. })) {
        console::error("error: --output-format json is not supported with task run");
        return 1;
    }

    // load dsconfig and tasks
    let context = match workspace_context(&args.program, None) {
        Ok(context) => context,
        Err(message) => {
            return report_error("task", &args.report, &message);
        }
    };
    let cwd = context.session.cwd.clone();
    let dsconfig_path = match resolve_dsconfig_path(&args.program, &context.resolver, &cwd) {
        Ok(path) => path,
        Err(message) => {
            return report_error("task", &args.report, &message);
        }
    };

    let tasks = match load_tasks(&dsconfig_path) {
        Ok(tasks) => tasks,
        Err(message) => {
            return report_error("task", &args.report, &message);
        }
    };

    // select command behavior
    match args.command.clone().unwrap_or(TaskCommand::List) {
        TaskCommand::List => {
            // emit structured output when requested
            if args.report.is_json() {
                let mut report = CommandReport::success("task", 0);
                report.data = Some(json!({
                    "tasks": tasks
                        .iter()
                        .map(|task| json!({
                            "name": task.name,
                            "description": task.description,
                        }))
                        .collect::<Vec<_>>(),
                }));
                print_report(&report, args.report.format());
            } else if tasks.is_empty() {
                console::info("task: no tasks defined");
            } else {
                for task in &tasks {
                    if let Some(description) = task.description.as_ref() {
                        console::info(&format!("{}: {}", task.name, description));
                    } else {
                        console::info(&task.name);
                    }
                }
            }
            0
        }
        TaskCommand::Run {
            name,
            args: task_args,
            dry_run,
        } => {
            // resolve task by name
            let task = match tasks.iter().find(|task| task.name == name) {
                Some(task) => task,
                None => {
                    return report_error("task", &args.report, &format!("task '{name}' not found"));
                }
            };

            // build final command line
            let mut command = task.command.clone();
            if !task_args.is_empty() {
                command.push(' ');
                command.push_str(&task_args.join(" "));
            }

            // return command when dry run is enabled
            if dry_run {
                console::info(&command);
                return 0;
            }

            // spawn task via shell
            let dsconfig_dir = dsconfig_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| dsconfig_path.clone());
            let mut shell = shell_command(&command);
            shell.current_dir(task.cwd.as_ref().unwrap_or(&dsconfig_dir));
            let status = shell.status();

            match status {
                Ok(status) => status.code().unwrap_or(1),
                Err(error) => {
                    console::error(&format!("task failed: {error}"));
                    1
                }
            }
        }
    }
}

/// Task specification loaded from dsconfig.json.
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
fn load_tasks(dsconfig_path: &PathBuf) -> Result<Vec<TaskSpec>, String> {
    // read the dsconfig file
    let content = std::fs::read_to_string(dsconfig_path)
        .map_err(|e| format!("failed to read {}: {e}", dsconfig_path.display()))?;

    // parse the config json
    let value: Value =
        serde_json::from_str(&content).map_err(|e| format!("invalid dsconfig: {e}"))?;

    // extract the task map
    let Some(tasks_value) = value.get("tasks") else {
        return Ok(Vec::new());
    };
    let tasks_object = tasks_value
        .as_object()
        .ok_or_else(|| "tasks must be an object".to_string())?;

    // resolve the dsconfig directory
    let dsconfig_dir = dsconfig_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| dsconfig_path.clone());

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

/// Build a shell command for task execution.
fn shell_command(command: &str) -> Command {
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
