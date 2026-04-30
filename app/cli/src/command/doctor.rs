use clap::Args;
use destack_daemon::protocol::{
    CommandDoctorOptions, CommandDoctorPayload, CommandDoctorToolStatus, CommandPayload,
};

use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev, report_from_payload};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_payload_command_or_report};

/// Arguments for the doctor command.
#[derive(Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Run extended checks and print more detail.
    #[arg(long)]
    pub full: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show environment and workspace diagnostics.
pub fn run(args: &DoctorArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("doctor", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Doctor(CommandDoctorOptions { full: args.full });

    run_root_payload_command_or_report::<CommandDoctorPayload, _, _>(
        "doctor",
        &args.report,
        &args.program,
        None,
        common,
        payload,
        "doctor",
        |exit_code, _, payload_value| {
            report_from_payload("doctor", exit_code, Some(payload_value), None, None)
        },
        |_, payload| {
            console::info(&format!("destack {}", payload.cli_version));
            console::info(&format!("cwd: {}", payload.cwd));
            console::info(&format!("os: {}", payload.os));
            console::info(&format!("arch: {}", payload.arch));
            console::info(&format!("workers: {}", payload.workers));
            console::info(&format!(
                "available parallelism: {}",
                payload.available_parallelism
            ));
            console::info(&format!(
                "workspace: {} ({})",
                payload.workspace.root, payload.workspace.kind
            ));
            console::info(&format!("packages: {}", payload.workspace.package_count));
            if let Some(packages) = payload.workspace.packages.as_ref() {
                for package in packages {
                    console::info(&format!("package: {package}"));
                }
            }
            if let Some(path) = payload.config.as_ref() {
                console::info(&format!("destack.json: {path}"));
            } else {
                console::warn("destack.json: not found");
            }
            if let Some(default_target) = payload.default_target.as_ref() {
                console::info(&format!("default target: {default_target}"));
            }
            if let Some(extends) = payload.extends.as_ref()
                && !extends.is_empty()
            {
                console::info(&format!("extends: {}", extends.join(", ")));
            }
            console::info(&format!("targets: {}", payload.target_count));
            if let Some(targets) = payload.targets.as_ref() {
                for name in targets {
                    console::info(&format!("target: {name}"));
                }
            }

            if let Some(tools) = payload.tools.as_ref()
                && !tools.is_empty()
            {
                console::info("tools:");
                for tool in tools {
                    match tool.status {
                        CommandDoctorToolStatus::Available => {
                            if let Some(version) = tool.version.as_ref() {
                                console::info(&format!("  {}: {version}", tool.name));
                            } else {
                                console::info(&format!("  {}: available", tool.name));
                            }
                        }
                        CommandDoctorToolStatus::Missing => {
                            console::warn(&format!("  {}: not found", tool.name));
                        }
                        CommandDoctorToolStatus::Error => {
                            console::warn(&format!("  {}: error", tool.name));
                        }
                    }
                }
            }

            if let Some(warnings) = payload.warnings.as_ref() {
                for warning in warnings {
                    console::warn(&format!("warning: {warning}"));
                }
            }
        },
    )
}
