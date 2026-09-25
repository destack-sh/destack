use crate::common::{
    CommandError, CommandOptionsBuilder, CommandResult, ListEntry, ListPrinter, ListSpacing,
    ProgramArgs, ReportArgs, command_error, ensure_no_watch_or_dev, list_payload, print_list_with,
    report_error, report_from_payload, run_workspace_payload_command_or_report,
};
use crate::console;
use clap::Args;
use tspp_workspace::{CommandRevision, TaskAction, TaskEntry, TaskInput, TaskPayload, TaskResult};

/// Arguments for the task command.
#[derive(Args, Debug, Clone)]
pub struct TaskArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Task name to run.
    pub name: Option<String>,

    /// Limit execution to selected projects.
    #[arg(long = "project", value_name = "PROJECT")]
    pub projects: Vec<String>,

    /// Limit execution to one named workspace group.
    #[arg(long = "group", value_name = "GROUP")]
    pub groups: Vec<String>,

    /// Arguments passed to the task.
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,

    /// Show the task command without executing.
    #[arg(long)]
    pub dry_run: bool,
}

/// Run workspace tasks.
pub async fn run(args: &TaskArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("task", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let action = match args.name.clone() {
        Some(name) => TaskAction::Run {
            name,
            args: args.args.clone(),
            dry_run: args.dry_run,
        },
        None => TaskAction::List,
    };
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("task", &args.report, &error.to_string()),
    };
    let request = TaskInput {
        action,
        projects: args.projects.clone(),
        groups: args.groups.clone(),
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<TaskPayload, _, _, _>(
        "task",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace.task(request, None).await.map_err(command_error)?;

            CommandResult::from_output(result)
        },
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
                "results": payload.results,
                "exit_code": exit_code,
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
                            let detail = task_list_detail(task);
                            if let Some(detail) = detail {
                                return entry.line(detail);
                            }

                            entry
                        })
                        .collect::<Vec<_>>();
                    let printer = ListPrinter::info();
                    print_list_with(&list_entries, ListSpacing::Compact, &printer);
                }
                return;
            }

            if let Some(results) = payload.results {
                for result in results {
                    let summary = task_result_summary(&result);
                    if result.exit_code.unwrap_or(default_exit_code) == 0 {
                        console::info(&summary);
                    } else {
                        console::warn(&summary);
                    }
                }
                return;
            }

            let exit_code = payload.exit_code.unwrap_or(default_exit_code);
            if exit_code != 0 {
                console::warn(&format!("process exited with code {exit_code}"));
            }
        },
    )
    .await
}

/// Build one display detail for one task list entry.
fn task_list_detail(task: &TaskEntry) -> Option<String> {
    let project_prefix = if task.project.is_empty() {
        String::new()
    } else {
        format!("[{}] ", task.project)
    };

    match task.description.as_deref() {
        Some(description) => Some(format!("{project_prefix}{description}")),
        None if !project_prefix.is_empty() => Some(project_prefix.trim_end().to_string()),
        None => None,
    }
}

/// Build one summary line for one task execution result.
fn task_result_summary(result: &TaskResult) -> String {
    let status_suffix = if result.dry_run {
        " (dry run)".to_string()
    } else if let Some(exit_code) = result.exit_code {
        format!(" (exit {exit_code})")
    } else {
        String::new()
    };

    format!("[{}] {}{}", result.project, result.task, status_suffix)
}
