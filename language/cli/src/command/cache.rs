use crate::common::{
    CommandOptionsBuilder, CommandResult, ListEntry, ListPrinter, ListSpacing, ProgramArgs,
    ReportArgs, command_error, ensure_no_watch_or_dev, list_payload, print_list_with, report_error,
    report_from_payload, run_workspace_payload_command_or_report,
};
use crate::console;
use clap::Args;
use destack_workspace::{CacheInput, CachePayload, CommandRevision};

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
pub async fn run(args: &CacheArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("cache", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("cache", &args.report, &error.to_string()),
    };
    let request = CacheInput {
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<CachePayload, _, _, _>(
        "cache",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace
                .cache(request, None)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
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
                let kind = entry.kind;
                let title = format!("{kind}: {location}");
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
    .await
}
