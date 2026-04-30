use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev, report_from_payload};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_payload_command_or_report};
use clap::Args;
use destack_daemon::protocol::{
    CommandInfoOptions, CommandInfoPayload, CommandInfoTarget, CommandPayload,
};

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

    run_root_payload_command_or_report::<CommandInfoPayload, _, _>(
        "info",
        &args.report,
        &args.program,
        None,
        common,
        payload,
        "info",
        |exit_code, _, payload_value| {
            report_from_payload("info", exit_code, Some(payload_value), None, None)
        },
        |_, payload| {
            console::info(&format!("workspace: {}", payload.workspace.root));
            console::info(&format!("kind: {}", payload.workspace.kind));
            for path in &payload.workspace.packages {
                console::info(&format!("package: {path}"));
            }
            if let Some(path) = payload.config.as_ref() {
                console::info(&format!("destack.json: {path}"));
            } else {
                console::warn("destack.json: not found");
            }
            if let Some(targets) = payload.targets.as_deref() {
                emit_targets("target", targets);
            }
            if let Some(workspace_targets) = payload.workspace_targets.as_deref()
                && !workspace_targets.is_empty()
            {
                emit_targets("workspace target", workspace_targets);
            }
        },
    )
}

/// Emit target details for an info payload section.
fn emit_targets(label: &str, targets: &[CommandInfoTarget]) {
    if targets.is_empty() {
        console::warn("targets: none");
        return;
    }

    for target in targets {
        console::info(&format!("{label}: {}", target.name));
        console::info(&format!("  emit: {}", target.emit));
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
