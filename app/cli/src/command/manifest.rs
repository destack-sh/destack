use std::path::PathBuf;

use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev, report_from_payload};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_payload_command_or_report};
use clap::Args;
use destack_daemon::protocol::{CommandManifestOptions, CommandManifestPayload, CommandPayload};

/// Arguments for the manifest command.
#[derive(Args, Debug, Clone)]
pub struct ManifestArgs {
    /// The manifest file or directory to inspect.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Show full manifest content in text mode.
    #[arg(long)]
    pub full: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show the resolved manifest.
pub fn run(args: &ManifestArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("manifest", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program).build();
    let payload = CommandPayload::Manifest(CommandManifestOptions {
        path: args.path.clone(),
        full: args.full,
    });

    run_root_payload_command_or_report::<CommandManifestPayload, _, _>(
        "manifest",
        &args.report,
        &args.program,
        common,
        payload,
        "manifest",
        |exit_code, _, payload_value| {
            report_from_payload("manifest", exit_code, Some(payload_value), None, None)
        },
        |_, payload| {
            console::info(&format!("manifest: {}", payload.path));
            if let Some(default_target) = payload.default_target.as_ref() {
                console::info(&format!("default target: {default_target}"));
            }
            if payload.targets.is_empty() {
                console::info("targets: none");
            } else {
                console::info(&format!("targets: {}", payload.targets.join(", ")));
            }

            // print full manifest text when requested
            if args.full
                && let Ok(pretty) = serde_json::to_string_pretty(&payload.manifest)
            {
                println!("{pretty}");
            }
        },
    )
}
