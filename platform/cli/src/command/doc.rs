use clap::Args;

use crate::common::{CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report};
use crate::console;

/// Arguments for the doc command.
#[derive(Args, Debug, Clone)]
pub struct DocArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Generate documentation.
pub fn run(args: &DocArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("doc", &args.program, &args.report) {
        return code;
    }

    // emit json placeholder when requested
    if args.report.is_json() {
        let mut report = CommandReport::failure("doc", 1);
        report.summary = Some("documentation generator is not implemented yet".to_string());
        print_report(&report, args.report.format());
        return 1;
    }

    // fall back to a minimal text error
    console::error("doc: not implemented yet");
    1
}
