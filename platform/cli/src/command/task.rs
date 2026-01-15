use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::json;

use crate::common::{
    CommandError, CommandReport, ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs,
    ensure_no_watch_or_dev, list_payload, print_list_with, print_report, report_error,
};
use crate::console;
use crate::pipeline::script::{load_tasks, shell_command};
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

/// Task entry returned by list output.
#[derive(serde::Serialize)]
struct TaskListEntry {
    /// Task name.
    name: String,
    /// Task description.
    description: Option<String>,
}

/// Run workspace tasks.
pub fn run(args: &TaskArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("task", &args.program, &args.report) {
        return code;
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

    let tasks = match load_tasks(&context.resolver, &dsconfig_path) {
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
                let entries = tasks
                    .iter()
                    .map(|task| TaskListEntry {
                        name: task.name.clone(),
                        description: task.description.clone(),
                    })
                    .collect();
                let mut report = CommandReport::success("task", 0);
                report.data = Some(list_payload(entries));
                print_report(&report, args.report.format());
            } else if tasks.is_empty() {
                console::info("task: no tasks defined");
            } else {
                let list_entries = tasks
                    .iter()
                    .map(|task| {
                        let entry = ListEntry::new(task.name.clone());
                        if let Some(description) = task.description.as_ref() {
                            entry.line(description.clone())
                        } else {
                            entry
                        }
                    })
                    .collect::<Vec<_>>();
                let printer = ListPrinter::info();
                print_list_with(&list_entries, ListSpacing::Compact, &printer);
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
                if args.report.is_json() {
                    let mut report = CommandReport::success("task", 0);
                    report.data = Some(json!({
                        "task": task.name,
                        "command": command,
                        "dry_run": true,
                    }));
                    print_report(&report, args.report.format());
                } else {
                    console::info(&command);
                }
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
                Ok(status) => {
                    let exit_code = status.code().unwrap_or(1);
                    if args.report.is_json() {
                        let mut report = if exit_code == 0 {
                            CommandReport::success("task", exit_code)
                        } else {
                            let message = format!("task exited with code {exit_code}");
                            let mut report = CommandReport::failure("task", exit_code);
                            report.summary = Some(message.clone());
                            report.error = Some(CommandError::new("task_exit", "task", message));
                            report
                        };
                        report.data = Some(json!({
                            "task": task.name,
                            "command": command,
                            "cwd": task
                                .cwd
                                .as_ref()
                                .unwrap_or(&dsconfig_dir)
                                .display()
                                .to_string(),
                            "exit_code": exit_code,
                            "dry_run": false,
                        }));
                        print_report(&report, args.report.format());
                    } else if exit_code != 0 {
                        console::warn(&format!("process exited with code {exit_code}"));
                    }
                    exit_code
                }
                Err(error) => {
                    if args.report.is_json() {
                        let message = format!("task failed: {error}");
                        let mut report = CommandReport::failure("task", 1);
                        report.summary = Some(message.clone());
                        report.error = Some(CommandError::new("task_failed", "task", message));
                        report.data = Some(json!({
                            "task": task.name,
                            "command": command,
                            "cwd": task
                                .cwd
                                .as_ref()
                                .unwrap_or(&dsconfig_dir)
                                .display()
                                .to_string(),
                            "exit_code": 1,
                            "dry_run": false,
                        }));
                        print_report(&report, args.report.format());
                    } else {
                        console::error(&format!("task failed: {error}"));
                    }
                    1
                }
            }
        }
    }
}

// task loading helpers live in pipeline::script
