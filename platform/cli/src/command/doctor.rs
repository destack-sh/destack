use clap::Args;
use destack_daemon::protocol::{
    CommandDoctorOptions, CommandDoctorPayload, CommandDoctorToolStatus, CommandPayload,
};

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, parse_required_command_payload,
    print_report, report_error,
};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, emit_daemon_text_output, run_daemon_command};

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

    // execute the daemon command
    let result = match run_daemon_command(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("doctor", &args.report, &error.to_string()),
    };

    // decode daemon payload for structured output
    let (payload, payload_value) = match parse_required_command_payload::<CommandDoctorPayload>(
        "doctor",
        &args.report,
        result.response.data.as_ref(),
        "doctor",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit daemon output for text mode
    emit_daemon_text_output(
        &args.report,
        &result.response.messages,
        &result.response.output,
    );

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("doctor", result.response.exit_code);
        report.data = Some(payload_value);
        print_report(&report, args.report.format());
        return result.response.exit_code;
    }

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
    if let Some(path) = payload.dsconfig.as_ref() {
        console::info(&format!("dsconfig: {path}"));
    } else {
        console::warn("dsconfig: not found");
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

    result.response.exit_code
}
