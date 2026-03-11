use crate::common::{
    CommandError, ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs,
    ensure_no_watch_or_dev, list_payload, print_list_with, report_from_payload,
};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_workspace_payload_command_or_report};
use clap::{Args, Subcommand};
use destack_daemon::protocol::{
    CommandPayload, CommandTaskAction, CommandTaskOptions, CommandTaskPayload,
};

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

    // build daemon command options
    let action = match args.command.clone().unwrap_or(TaskCommand::List) {
        TaskCommand::List => CommandTaskAction::List,
        TaskCommand::Run {
            name,
            args,
            dry_run,
        } => CommandTaskAction::Run {
            name,
            args,
            dry_run,
        },
    };
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Task(CommandTaskOptions { action });

    run_workspace_payload_command_or_report::<CommandTaskPayload, _, _>(
        "task",
        &args.report,
        &args.program,
        None,
        common,
        payload,
        "task",
        |default_exit_code, payload, _| {
            if let Some(tasks) = payload.tasks {
                return report_from_payload(
                    "task",
                    default_exit_code,
                    Some(list_payload(tasks)),
                    None,
                    None,
                );
            }

            let exit_code = payload.exit_code.unwrap_or(default_exit_code);
            let data = serde_json::json!({
                "task": payload.task.unwrap_or_default(),
                "command": payload.command.unwrap_or_default(),
                "cwd": payload.cwd,
                "exit_code": exit_code,
                "dry_run": payload.dry_run.unwrap_or(false),
            });
            if exit_code == 0 {
                return report_from_payload("task", exit_code, Some(data), None, None);
            }

            let message = format!("task exited with code {exit_code}");
            report_from_payload(
                "task",
                exit_code,
                Some(data),
                Some(message.clone()),
                Some(CommandError::new("task_exit", "task", message)),
            )
        },
        |default_exit_code, payload| {
            if let Some(tasks) = payload.tasks {
                if tasks.is_empty() {
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
                return;
            }

            let exit_code = payload.exit_code.unwrap_or(default_exit_code);
            if exit_code != 0 {
                console::warn(&format!("process exited with code {exit_code}"));
            }
        },
    )
}
