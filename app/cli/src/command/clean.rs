use std::path::PathBuf;

use clap::Args;

use crate::common::{
    CommandError, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, parse_command_payload,
    print_report, report_from_payload,
};
use crate::console;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, emit_daemon_text_output, run_root_command_or_report,
};
use destack_daemon::protocol::{CommandCleanOptions, CommandCleanPayload, CommandPayload};

/// Arguments for the clean command.
#[derive(Args, Debug, Clone)]
pub struct CleanArgs {
    /// The directory to clean (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Remove build output directories.
    #[arg(long)]
    pub dist: bool,

    /// Remove cache directories.
    #[arg(long)]
    pub cache: bool,

    /// Remove all build outputs and caches.
    #[arg(long)]
    pub all: bool,

    /// Clean all packages in the workspace.
    #[arg(long = "all-packages", alias = "workspace-all")]
    pub all_packages: bool,

    /// Show what would be removed without deleting.
    #[arg(long)]
    pub dry_run: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Remove build outputs and caches.
pub fn run(args: &CleanArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("clean", &args.program, &args.report) {
        return code;
    }

    run_clean_via_daemon(args)
}

/// Run a clean command through the daemon.
fn run_clean_via_daemon(args: &CleanArgs) -> i32 {
    // build command options for daemon execution
    let clean = CommandCleanOptions {
        dir: args.dir.clone(),
        dist: args.dist,
        cache: args.cache,
        all: args.all,
        all_packages: args.all_packages,
    };
    let common = CommandOptionsBuilder::new(&args.program, None)
        .dry_run(args.dry_run)
        .build();
    let payload = CommandPayload::Clean(clean);

    // execute the daemon command
    let result = match run_root_command_or_report(
        "clean",
        &args.report,
        &args.program,
        None,
        common,
        payload,
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit daemon output and messages for text modes
    if !args.report.is_json() {
        emit_daemon_text_output(
            &args.report,
            &result.response.messages,
            &result.response.output,
        );
    }

    // decode payload for structured output
    let payload = match parse_command_payload::<CommandCleanPayload>(
        "clean",
        &args.report,
        result.response.data.as_ref(),
        "clean",
        false,
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let exit_code = result.response.exit_code;
    if args.report.is_json() {
        let summary = if exit_code == 0 {
            None
        } else {
            Some("clean encountered errors".to_string())
        };
        let error = if exit_code == 0 {
            None
        } else {
            Some(CommandError::new(
                "clean_failed",
                "clean",
                "clean encountered errors",
            ))
        };
        let report = report_from_payload(
            "clean",
            exit_code,
            payload.as_ref().map(|(_, value)| value.clone()),
            summary,
            error,
        );
        print_report(&report, args.report.format());
    }

    if exit_code != 0 && !args.report.is_json() {
        console::warn(&format!("clean exited with code {exit_code}"));
    }

    exit_code
}
