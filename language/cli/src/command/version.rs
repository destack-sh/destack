use clap::Args;

use crate::common::{ReportArgs, print_json_payload_report};
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

/// JSON payload for version output.
#[derive(serde::Serialize)]
struct VersionPayload {
    /// TS++ version identifier.
    tspp: String,
    /// CLI version identifier.
    cli: String,
}

/// Show version information.
pub fn run(args: &VersionArgs) -> i32 {
    // emit structured output when requested
    if args.report.is_json() {
        let payload = VersionPayload {
            tspp: CLI_VERSION.to_string(),
            cli: CLI_VERSION.to_string(),
        };
        if let Err(code) = print_json_payload_report("version", &args.report, 0, &payload) {
            return code;
        }
        return 0;
    }

    // emit minimal text output
    console::info(&format!("tspp {CLI_VERSION}"));
    0
}
