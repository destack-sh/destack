use std::path::PathBuf;

use clap::Args;
use destack_daemon::protocol::{CommandFormatOptions, CommandFormatPayload, CommandPayload};

use crate::common::{
    DiagnosticArgs, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, parse_command_payload,
    print_report, report_from_payload,
};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, emit_daemon_text_output, run_root_command_or_report,
};

/// Arguments for the format command.
#[derive(Args, Debug, Clone)]
pub struct FmtArgs {
    /// Input files or directories to format.
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,

    /// Format inline string (output to stdout).
    #[arg(short = 'e', long = "eval")]
    pub eval: Option<String>,

    /// Check if files are formatted (exit 1 if not, don't write).
    #[arg(long)]
    pub check: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Format source files.
pub fn run(args: &FmtArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("fmt", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program).build();
    let payload = CommandPayload::Format(CommandFormatOptions {
        files: args.files.clone(),
        eval: args.eval.clone(),
        check: args.check,
    });

    // execute the daemon command
    let result =
        match run_root_command_or_report("fmt", &args.report, &args.program, common, payload) {
            Ok(result) => result,
            Err(code) => return code,
        };

    // emit daemon output for text mode
    if !args.report.is_json() {
        emit_daemon_text_output(
            &args.report,
            &result.response.messages,
            &result.response.output,
        );
        return result.response.exit_code;
    }

    // emit structured report for json mode
    let exit_code = result.response.exit_code;
    let payload = match parse_command_payload::<CommandFormatPayload>(
        "fmt",
        &args.report,
        result.response.data.as_ref(),
        "format",
        false,
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let summary = if payload.is_some() {
        None
    } else {
        Some("format payload missing".to_string())
    };
    let report = report_from_payload(
        "fmt",
        exit_code,
        payload.as_ref().map(|(_, value)| value.clone()),
        summary,
        None,
    );
    print_report(&report, args.report.format());
    exit_code
}
