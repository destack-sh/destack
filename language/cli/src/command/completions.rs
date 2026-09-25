use std::io::Write;

use clap::Args;
use clap_complete::{Shell, generate};

use crate::common::{ReportArgs, print_json_payload_report};
use crate::console;

use crate::app::{HelpMode, build_command};

/// Arguments for the completions command.
#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    /// The shell to generate completions for.
    #[arg(value_enum)]
    pub shell: Shell,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// JSON payload for completion scripts.
#[derive(serde::Serialize)]
struct CompletionsPayload {
    /// Shell name for the completion script.
    shell: String,
    /// Completion script content.
    script: String,
}

/// Generate shell completion scripts.
pub fn run(args: &CompletionsArgs) -> i32 {
    // generate the completion script into a buffer
    let mut command = build_command(HelpMode::Full);
    let mut buffer = Vec::new();
    generate(args.shell, &mut command, "tspp", &mut buffer);

    // emit structured output when requested
    if args.report.is_json() {
        let script = String::from_utf8_lossy(&buffer).to_string();
        let payload = CompletionsPayload {
            shell: args.shell.to_string(),
            script,
        };
        if let Err(code) = print_json_payload_report("completions", &args.report, 0, &payload) {
            return code;
        }
        return 0;
    }

    // write the script to stdout
    if let Err(error) = std::io::stdout().write_all(&buffer) {
        console::error(&format!("failed to write completions: {error}"));
        return 1;
    }
    0
}
