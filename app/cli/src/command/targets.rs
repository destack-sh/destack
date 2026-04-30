use crate::common::{
    ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs, ensure_no_watch_or_dev,
    list_payload, print_list_with, report_from_payload,
};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_payload_command_or_report};
use clap::Args;
use destack_daemon::protocol::{CommandPayload, CommandTargetsOptions, CommandTargetsPayload};

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
pub fn run(args: &TargetsArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("targets", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Targets(CommandTargetsOptions { all: args.all });

    run_root_payload_command_or_report::<CommandTargetsPayload, _, _>(
        "targets",
        &args.report,
        &args.program,
        None,
        common,
        payload,
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
                let default_tag = entry
                    .default_target
                    .as_ref()
                    .map(|target| {
                        if target == &entry.name {
                            " (default)"
                        } else {
                            ""
                        }
                    })
                    .unwrap_or("");
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
}
