use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, parse_required_command_payload,
    print_report, report_error,
};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, emit_daemon_text_output, run_daemon_command};
use clap::Args;
use destack_daemon::protocol::{CommandInfoOptions, CommandInfoPayload, CommandPayload};

/// Arguments for the info command.
#[derive(Args, Debug, Clone)]
pub struct InfoArgs {
    /// Show info for all workspace packages.
    #[arg(long)]
    pub all: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show workspace and target information.
pub fn run(args: &InfoArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("info", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Info(CommandInfoOptions { all: args.all });

    // execute the daemon command
    let result = match run_daemon_command(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("info", &args.report, &error.to_string()),
    };

    // decode daemon payload for structured output
    let (payload, payload_value) = match parse_required_command_payload::<CommandInfoPayload>(
        "info",
        &args.report,
        result.response.data.as_ref(),
        "info",
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
        let mut report = CommandReport::success("info", result.response.exit_code);
        report.data = Some(payload_value);
        print_report(&report, args.report.format());
        return result.response.exit_code;
    }

    // emit minimal text output
    console::info(&format!("workspace: {}", payload.workspace.root));
    console::info(&format!("kind: {}", payload.workspace.kind));
    for path in &payload.workspace.packages {
        console::info(&format!("package: {path}"));
    }
    if let Some(path) = payload.dsconfig.as_ref() {
        console::info(&format!("dsconfig: {path}"));
    } else {
        console::warn("dsconfig: not found");
    }
    if let Some(targets) = payload.targets.as_ref() {
        if targets.is_empty() {
            console::warn("targets: none");
        } else {
            // emit target details for the active package
            for target in targets {
                console::info(&format!("target: {}", target.name));
                console::info(&format!("  output: {}", target.output));
                console::info(&format!("  runtime: {}", target.runtime));
                console::info(&format!("  platform: {}", target.platform));
                console::info(&format!("  out_dir: {}", target.out_dir));
                if let Some(out_file) = target.out_file.as_ref() {
                    console::info(&format!("  out_file: {out_file}"));
                }
                if let Some(package_dir) = target.package_dir.as_ref() {
                    console::info(&format!("  package_dir: {package_dir}"));
                }
            }
        }
    }

    if let Some(workspace_targets) = payload.workspace_targets.as_ref()
        && !workspace_targets.is_empty()
    {
        // emit target details for the workspace packages
        for target in workspace_targets {
            console::info(&format!("workspace target: {}", target.name));
            console::info(&format!("  output: {}", target.output));
            console::info(&format!("  runtime: {}", target.runtime));
            console::info(&format!("  platform: {}", target.platform));
            console::info(&format!("  out_dir: {}", target.out_dir));
            if let Some(out_file) = target.out_file.as_ref() {
                console::info(&format!("  out_file: {out_file}"));
            }
            if let Some(package_dir) = target.package_dir.as_ref() {
                console::info(&format!("  package_dir: {package_dir}"));
            }
        }
    }

    result.response.exit_code
}
