use crate::common::{
    CommandReport, ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs,
    ensure_no_watch_or_dev, list_payload, parse_required_command_payload, print_list_with,
    print_report, report_error,
};
use crate::console;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, emit_daemon_text_output, run_workspace_command_once,
};
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

    // execute the daemon command
    let result = match run_workspace_command_once(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("targets", &args.report, &error.to_string()),
    };

    // decode daemon payload
    let (payload, _) = match parse_required_command_payload::<CommandTargetsPayload>(
        "targets",
        &args.report,
        result.response.data.as_ref(),
        "targets",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let entries = payload.targets;

    // emit daemon output for text mode
    emit_daemon_text_output(
        &args.report,
        &result.response.messages,
        &result.response.output,
    );

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("targets", result.response.exit_code);
        report.data = Some(list_payload(entries));
        print_report(&report, args.report.format());
        return result.response.exit_code;
    }

    // emit minimal text output
    if entries.is_empty() {
        console::info("targets: none");
        return result.response.exit_code;
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
            entry.name, default_tag, entry.output, entry.runtime, entry.platform
        );
        ListEntry::new(title)
    });
    let list_entries: Vec<ListEntry> = list_entries.collect();
    let printer = ListPrinter::info();
    print_list_with(&list_entries, ListSpacing::Compact, &printer);

    result.response.exit_code
}
