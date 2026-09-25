use clap::Args;
use tspp_workspace::{CommandRevision, DoctorInput, DoctorPayload, DoctorToolStatus};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, report_error, report_from_payload,
    run_workspace_payload_command_or_report,
};
use crate::console;

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
pub async fn run(args: &DoctorArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("doctor", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("doctor", &args.report, &error.to_string()),
    };
    let request = DoctorInput {
        full: args.full,
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<DoctorPayload, _, _, _>(
        "doctor",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace
                .doctor(request, None)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
        "doctor",
        |exit_code, _, payload_value| {
            report_from_payload("doctor", exit_code, Some(payload_value), None, None)
        },
        |_, payload| {
            console::info(&format!("tspp {}", payload.cli_version));
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
            if let Some(path) = payload.manifest.as_ref() {
                console::info(&format!("package.json: {path}"));
            } else {
                console::warn("package.json: not found");
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
                        DoctorToolStatus::Available => {
                            if let Some(version) = tool.version.as_ref() {
                                console::info(&format!("  {}: {version}", tool.name));
                            } else {
                                console::info(&format!("  {}: available", tool.name));
                            }
                        }
                        DoctorToolStatus::Missing => {
                            console::warn(&format!("  {}: not found", tool.name));
                        }
                        DoctorToolStatus::Error => {
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
    .await
}
