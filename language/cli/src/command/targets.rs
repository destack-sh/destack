use crate::common::{
    CommandOptionsBuilder, CommandResult, ListEntry, ListPrinter, ListSpacing, ProgramArgs,
    ReportArgs, command_error, ensure_no_watch_or_dev, list_payload, print_list_with, report_error,
    report_from_payload, run_workspace_payload_command_or_report,
};
use crate::console;
use clap::Args;
use tspp_workspace::{CommandRevision, TargetsInput, TargetsPayload};

/// Arguments for the targets command.
#[derive(Args, Debug, Clone)]
pub struct TargetsArgs {
    /// List targets for all workspace packages.
    #[arg(long)]
    pub all: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// List configured build targets.
pub async fn run(args: &TargetsArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("targets", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("targets", &args.report, &error.to_string()),
    };
    let request = TargetsInput {
        all: args.all,
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<TargetsPayload, _, _, _>(
        "targets",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace
                .targets(request, None)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
        "targets",
        |exit_code, payload, _| {
            report_from_payload(
                "targets",
                exit_code,
                Some(list_payload(payload.targets)),
                None,
                None,
            )
        },
        |_, payload| {
            let entries = payload.targets;
            if entries.is_empty() {
                console::info("targets: none");
                return;
            }

            let list_entries = entries.into_iter().map(|entry| {
                let default_tag = if entry.is_default { " (default)" } else { "" };
                let title = format!(
                    "{}{}  [{} | {} | {}]",
                    entry.name, default_tag, entry.emit, entry.runtime, entry.platform
                );
                ListEntry::new(title)
            });
            let list_entries: Vec<ListEntry> = list_entries.collect();
            let printer = ListPrinter::info();
            print_list_with(&list_entries, ListSpacing::Compact, &printer);
        },
    )
    .await
}
