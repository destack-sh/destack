use clap::Args;

use crate::common::{CommandReport, ReportArgs, print_report};
use crate::console;

/// CLI version from Cargo metadata.
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Arguments for the version command.
#[derive(Args, Debug, Clone)]
pub struct VersionArgs {
    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show version information.
pub fn run(args: &VersionArgs) -> i32 {
    // emit structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("version", 0);
        report.data = Some(serde_json::json!({
            "destack": CLI_VERSION,
            "cli": CLI_VERSION,
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    // emit minimal text output
    console::info(&format!("destack {CLI_VERSION}"));
    0
}
