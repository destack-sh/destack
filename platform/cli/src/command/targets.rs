use clap::Args;

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report, report_error,
};
use crate::console;
use crate::pipeline::workspace::{
    load_dsconfig_for_program, load_workspace_dsconfigs, workspace_context,
};

/// Arguments for the targets command.
#[derive(Args, Debug, Clone)]
pub struct TargetsArgs {
    /// List targets for all workspace packages.
    #[arg(long)]
    pub all: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// List configured build targets.
pub fn run(args: &TargetsArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("targets", &args.program, &args.report) {
        return code;
    }

    // set up workspace context
    let context = match workspace_context(&args.program, None) {
        Ok(context) => context,
        Err(message) => return report_error("targets", &args.report, &message),
    };

    // resolve dsconfig selection
    let dsconfigs = if args.all {
        match load_workspace_dsconfigs(&context.resolver, &context.workspace) {
            Ok(dsconfigs) => dsconfigs,
            Err(message) => return report_error("targets", &args.report, &message),
        }
    } else {
        match load_dsconfig_for_program(&args.program, &context.resolver, &context.session.cwd) {
            Ok(dsconfig) => vec![dsconfig],
            Err(message) => return report_error("targets", &args.report, &message),
        }
    };

    if dsconfigs.is_empty() {
        return report_error("targets", &args.report, "no targets found");
    }

    // collect target details
    let mut entries = Vec::new();
    for dsconfig in &dsconfigs {
        let default_target = dsconfig.options.default_target.clone();
        for (name, target) in &dsconfig.options.targets {
            entries.push(TargetEntry {
                name: name.clone(),
                output: format!("{:?}", target.output),
                runtime: format!("{:?}", target.runtime),
                platform: format!("{:?}", target.platform),
                out_dir: target.out_dir.display().to_string(),
                out_file: target.out_file.as_ref().map(|p| p.display().to_string()),
                default_target: default_target.clone(),
                package_dir: dsconfig.directory.display().to_string(),
            });
        }
    }

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("targets", 0);
        report.data = Some(serde_json::json!({
            "targets": entries,
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    // emit minimal text output
    if entries.is_empty() {
        console::info("targets: none");
        return 0;
    }

    for entry in entries {
        let default_tag = entry
            .default_target
            .as_ref()
            .map(|target| {
                if target == &entry.name {
                    " (default)"
                } else {
                    ""
                }
            })
            .unwrap_or("");
        console::info(&format!(
            "{}{}  [{} | {} | {}]",
            entry.name, default_tag, entry.output, entry.runtime, entry.platform
        ));
    }

    0
}

/// Target listing entry.
#[derive(Debug, serde::Serialize)]
struct TargetEntry {
    /// The target name.
    name: String,
    /// The output format.
    output: String,
    /// The runtime environment.
    runtime: String,
    /// The target platform.
    platform: String,
    /// The output directory.
    out_dir: String,
    /// The output file path, when applicable.
    out_file: Option<String>,
    /// The default target name for the package.
    default_target: Option<String>,
    /// The owning package directory.
    package_dir: String,
}
