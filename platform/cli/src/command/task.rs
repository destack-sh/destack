use crate::common::{
    CommandError, CommandReport, ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs,
    ensure_no_watch_or_dev, list_payload, parse_required_command_payload, print_list_with,
    print_report, report_error,
};
use crate::console;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, emit_daemon_text_output, run_workspace_command_once,
};
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

    // execute the daemon command
    let result = match run_workspace_command_once(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("task", &args.report, &error.to_string()),
    };

    // decode daemon payload
    let (payload, _) = match parse_required_command_payload::<CommandTaskPayload>(
        "task",
        &args.report,
        result.response.data.as_ref(),
        "task",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit daemon messages and output for text mode
    emit_daemon_text_output(
        &args.report,
        &result.response.messages,
        &result.response.output,
    );

    // handle list output
    if let Some(tasks) = payload.tasks {
        if args.report.is_json() {
            let mut report = CommandReport::success("task", result.response.exit_code);
            report.data = Some(list_payload(tasks));
            print_report(&report, args.report.format());
            return result.response.exit_code;
        }

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
        return result.response.exit_code;
    }

    // handle run output
    let exit_code = payload.exit_code.unwrap_or(result.response.exit_code);
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
        let task_name = payload.task.unwrap_or_default();
        let command = payload.command.unwrap_or_default();
        report.data = Some(serde_json::json!({
            "task": task_name,
            "command": command,
            "cwd": payload.cwd,
            "exit_code": exit_code,
            "dry_run": payload.dry_run.unwrap_or(false),
        }));
        print_report(&report, args.report.format());
        return exit_code;
    }

    if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }

    exit_code
}
