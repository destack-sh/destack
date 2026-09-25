use std::path::PathBuf;

use clap::Args;
use serde::Serialize;
use tspp_artifact::{ArtifactCache, ArtifactCacheRemoval};
use tspp_workspace::{CleanInput, CleanPayload, CommandRevision};

use crate::common::{
    CommandError, CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    emit_workspace_text_output, ensure_no_watch_or_dev, parse_required_command_payload,
    print_report, report_error, report_from_payload, run_workspace_command_or_report,
};
use crate::console;

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

/// Machine-readable workspace and cache clean result.
#[derive(Serialize)]
struct CleanReport {
    /// Workspace output removal.
    #[serde(flatten)]
    workspace: CleanPayload,
    /// Machine cache removal when selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    cache: Option<ArtifactCacheRemoval>,
}

/// Remove build outputs and caches.
pub async fn run(args: &CleanArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("clean", &args.program, &args.report) {
        return code;
    }

    run_clean(args).await
}

/// Run a clean command.
async fn run_clean(args: &CleanArgs) -> i32 {
    let is_workspace_selected = args.dist || args.all || !args.cache;
    let is_cache_selected = args.cache || args.all;

    // clear only the machine cache without opening a workspace
    if !is_workspace_selected {
        return run_cache_clean(args);
    }

    // build command options for workspace execution
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common,
        Err(error) => return report_error("clean", &args.report, &error.to_string()),
    }
    .dry_run(args.dry_run)
    .build();
    let request = CleanInput {
        dir: args.dir.clone(),
        all_packages: args.all_packages,
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "clean",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace
                .clean(request, None)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    )
    .await
    {
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
    let (workspace, _) = match parse_required_command_payload::<CleanPayload>(
        "clean",
        &args.report,
        Some(&result.response.data),
        "clean",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let mut clean = CleanReport {
        workspace,
        cache: None,
    };
    let mut exit_code = result.response.exit_code;

    // print each workspace removal and error
    if !args.report.is_json() {
        let action = if clean.workspace.dry_run {
            "Would remove"
        } else {
            "Removed"
        };
        for path in &clean.workspace.removed {
            console::status(action, path);
        }
        for error in &clean.workspace.errors {
            console::error(&format!("error: {error}"));
        }
    }

    // clear the machine cache after the workspace writer has closed
    if is_cache_selected {
        match clear_cache(args) {
            Ok(removal) => {
                clean.cache = Some(removal);
                if !args.report.is_json() {
                    print_cache_removal(removal, args.dry_run);
                }
            }
            Err(error) => {
                clean.workspace.errors.push(error.clone());
                if !args.report.is_json() {
                    console::error(&format!("error: {error}"));
                }
                exit_code = 1;
            }
        }
    }

    // emit one combined machine-readable result
    if args.report.is_json() {
        let data = match serde_json::to_value(clean) {
            Ok(data) => Some(data),
            Err(error) => {
                return report_error("clean", &args.report, &error.to_string());
            }
        };
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
        let mut report = report_from_payload("clean", exit_code, data, summary, error);
        report.trace = result.response.trace.clone();
        print_report(&report, args.report.format());
    }

    // finish human-readable output
    if !args.report.is_json() {
        if exit_code != 0 {
            console::warn(&format!("clean exited with code {exit_code}"));
        }
        result.emit_timings(None);
    }

    exit_code
}

/// Clear only the machine artifact cache.
fn run_cache_clean(args: &CleanArgs) -> i32 {
    match clear_cache(args) {
        Ok(removal) => {
            if args.report.is_json() {
                let workspace = CleanPayload {
                    removed: Vec::new(),
                    errors: Vec::new(),
                    dry_run: args.dry_run,
                };
                let clean = CleanReport {
                    workspace,
                    cache: Some(removal),
                };
                let data = match serde_json::to_value(clean) {
                    Ok(data) => data,
                    Err(error) => return report_error("clean", &args.report, &error.to_string()),
                };
                let report = report_from_payload("clean", 0, Some(data), None, None);
                print_report(&report, args.report.format());
            } else {
                print_cache_removal(removal, args.dry_run);
            }

            0
        }
        Err(error) => report_error("clean", &args.report, &error),
    }
}

/// Clear the configured machine artifact cache.
fn clear_cache(args: &CleanArgs) -> Result<ArtifactCacheRemoval, String> {
    let (directory, _) = args
        .program
        .resolve_artifact_cache()
        .map_err(|error| error.to_string())?;
    let removal =
        ArtifactCache::clear(&directory, args.dry_run).map_err(|error| error.to_string())?;

    Ok(removal)
}

/// Print one cache removal result.
fn print_cache_removal(removal: ArtifactCacheRemoval, is_dry_run: bool) {
    let action = if is_dry_run {
        "Would remove"
    } else {
        "Removed"
    };
    let bytes = console::format_bytes(removal.bytes);
    let files = removal.files;
    console::status(action, &format!("{bytes} in {files} files"));
}
