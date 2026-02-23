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
use destack_daemon::protocol::{CommandCacheOptions, CommandCachePayload, CommandPayload};

/// Arguments for the cache command.
#[derive(Args, Debug, Clone)]
pub struct CacheArgs {
    /// Show cache locations for all workspace packages.
    #[arg(long = "all-packages", alias = "workspace-all")]
    pub all_packages: bool,

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
    let payload = CommandPayload::Cache(CommandCacheOptions {
        all_packages: args.all_packages,
    });

    // execute the daemon command
    let result = match run_workspace_command_once(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("cache", &args.report, &error.to_string()),
    };

    // decode daemon payload
    let (payload, _) = match parse_required_command_payload::<CommandCachePayload>(
        "cache",
        &args.report,
        result.response.data.as_ref(),
        "cache",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit daemon output for text mode
    emit_daemon_text_output(
        &args.report,
        &result.response.messages,
        &result.response.output,
    );

    if args.report.is_json() {
        let mut report = CommandReport::success("cache", result.response.exit_code);
        report.data = Some(list_payload(payload.caches));
        print_report(&report, args.report.format());
        return result.response.exit_code;
    }

    let list_entries = payload.caches.into_iter().map(|entry| {
        let location = entry.dir;
        let source = entry.source;
        let title = if let Some(package_dir) = entry.package_dir {
            format!("{location} ({source}, package: {package_dir})")
        } else {
            format!("{location} ({source})")
        };
        ListEntry::new(title)
    });
    let list_entries: Vec<ListEntry> = list_entries.collect();
    if list_entries.is_empty() {
        console::info("cache: no entries");
        return result.response.exit_code;
    }
    let printer = ListPrinter::info();
    print_list_with(&list_entries, ListSpacing::Compact, &printer);

    result.response.exit_code
}
