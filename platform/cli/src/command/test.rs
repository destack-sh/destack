use clap::Args;

use crate::common::{
    CommandError, CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report,
};
use crate::console;

/// Arguments for the test command.
#[derive(Args, Debug, Clone)]
pub struct TestArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Run tests.
pub fn run(args: &TestArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("test", &args.program, &args.report) {
        return code;
    }

    // emit json placeholder when requested
    if args.report.is_json() {
        let message = "test runner is not implemented yet";
        let mut report = CommandReport::failure("test", 1);
        report.summary = Some(message.to_string());
        report.error = Some(CommandError::new("not_implemented", "feature", message));
        print_report(&report, args.report.format());
        return 1;
    }

    // fall back to a minimal text error
    console::error("test: not implemented yet");
    1
}
