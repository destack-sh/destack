use clap::Args;

use crate::common::{
    CommandError, CommandReport, DiagnosticArgs, ProgramArgs, ReportArgs, TargetArgs,
    ensure_no_watch_or_dev, print_report,
};
use crate::console;

/// Arguments for the repl command.
#[derive(Args, Debug, Clone)]
pub struct ReplArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Target configuration.
    #[command(flatten)]
    pub target: TargetArgs,

    /// Diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Start a REPL session.
pub fn run(args: &ReplArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("repl", &args.program, &args.report) {
        return code;
    }

    // emit json placeholder when requested
    if args.report.is_json() {
        let message = "repl is not implemented yet";
        let mut report = CommandReport::failure("repl", 1);
        report.summary = Some(message.to_string());
        report.error = Some(CommandError::new("not_implemented", "feature", message));
        print_report(&report, args.report.format());
        return 1;
    }

    // fall back to a minimal text error
    console::error("repl: not implemented yet");
    1
}
