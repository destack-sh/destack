use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, report_error, report_from_payload,
    run_workspace_payload_command_or_report,
};
use crate::console;
use clap::Args;
use tspp_workspace::{CommandRevision, InfoInput, InfoPayload, TargetEntry};

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
pub async fn run(args: &InfoArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("info", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("info", &args.report, &error.to_string()),
    };
    let request = InfoInput {
        all: args.all,
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<InfoPayload, _, _, _>(
        "info",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace.info(request, None).await.map_err(command_error)?;

            CommandResult::from_output(result)
        },
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
            if let Some(path) = payload.manifest.as_ref() {
                console::info(&format!("package.json: {path}"));
            } else {
                console::warn("package.json: not found");
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
    .await
}

/// Emit target details for an info payload section.
fn emit_targets(label: &str, targets: &[TargetEntry]) {
    if targets.is_empty() {
        console::warn("targets: none");
        return;
    }

    for target in targets {
        let suffix = if target.is_default { " (default)" } else { "" };

        console::info(&format!("{label}: {}{suffix}", target.name));
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
