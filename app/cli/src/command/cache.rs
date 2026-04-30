use crate::common::{
    ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs, ensure_no_watch_or_dev,
    list_payload, print_list_with, report_from_payload,
};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_payload_command_or_report};
use clap::Args;
use destack_daemon::protocol::{CommandCacheOptions, CommandCachePayload, CommandPayload};

/// Arguments for the cache command.
#[derive(Args, Debug, Clone)]
pub struct CacheArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show cache directory locations.
pub fn run(args: &CacheArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("cache", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Cache(CommandCacheOptions);

    run_root_payload_command_or_report::<CommandCachePayload, _, _>(
        "cache",
        &args.report,
        &args.program,
        None,
        common,
        payload,
        "cache",
        |exit_code, payload, _| {
            report_from_payload(
                "cache",
                exit_code,
                Some(list_payload(payload.caches)),
                None,
                None,
            )
        },
        |_, payload| {
            let list_entries = payload.caches.into_iter().map(|entry| {
                let location = entry.directory;
                let source = entry.source;
                let title = format!("{location} ({source})");
                ListEntry::new(title)
            });
            let list_entries: Vec<ListEntry> = list_entries.collect();
            if list_entries.is_empty() {
                console::info("cache: no entries");
                return;
            }
            let printer = ListPrinter::info();
            print_list_with(&list_entries, ListSpacing::Compact, &printer);
        },
    )
}
