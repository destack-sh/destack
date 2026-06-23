use std::path::PathBuf;

use clap::Args;

use crate::common::{
    CommandError, CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    emit_workspace_text_output, ensure_no_watch_or_dev, parse_command_payload, print_report,
    report_error, report_from_payload, run_workspace_command_or_report,
};
use crate::console;
use destack_workspace::{CleanInput, CleanPayload, CommandRevision};

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

    run_clean(args)
}

/// Run a clean command.
fn run_clean(args: &CleanArgs) -> i32 {
    // build command options for workspace execution
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common,
        Err(error) => return report_error("clean", &args.report, &error.to_string()),
    }
    .dry_run(args.dry_run)
    .build();
    let request = CleanInput {
        dir: args.dir.clone(),
        dist: args.dist,
        cache: args.cache,
        all: args.all,
        all_packages: args.all_packages,
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "clean",
        &args.report,
        &args.program,
        |workspace, root, _| {
            let result = workspace
                .clean(root, request, None)
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit workspace output and messages for text modes
    if !args.report.is_json() {
        emit_workspace_text_output(
            &args.report,
            &result.response.messages,
            &result.response.output,
        );
    }

    // decode payload for structured output
    let payload = match parse_command_payload::<CleanPayload>(
        "clean",
        &args.report,
        Some(&result.response.data),
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
