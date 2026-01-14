use clap::Args;
use serde_json::json;

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report, report_error,
};
use crate::console;
use crate::pipeline::workspace::{
    find_dsconfig, load_dsconfig, load_workspace_dsconfigs, resolve_dsconfig_path,
    workspace_context,
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

    // set up workspace context
    let context = match workspace_context(&args.program, None) {
        Ok(context) => context,
        Err(message) => return report_error("info", &args.report, &message),
    };
    let cwd = context.session.cwd.clone();

    // load config for cwd
    let dsconfig_path = if args.program.config.is_some() {
        match resolve_dsconfig_path(&args.program, &context.resolver, &cwd) {
            Ok(path) => Some(path),
            Err(message) => return report_error("info", &args.report, &message),
        }
    } else {
        find_dsconfig(&context.resolver, &cwd)
    };
    let dsconfig = dsconfig_path
        .as_ref()
        .and_then(|path| load_dsconfig(&context.resolver, path).ok());

    // load workspace configs when requested
    let workspace_configs = if args.all {
        load_workspace_dsconfigs(&context.resolver, &context.workspace).ok()
    } else {
        None
    };

    let workspace = &context.workspace;
    let package_paths: Vec<String> = workspace
        .package_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    // derive target summaries
    let targets = dsconfig.as_ref().map(|config| {
        config
            .options
            .targets
            .iter()
            .map(|(name, target)| {
                json!({
                    "name": name,
                    "output": format!("{:?}", target.output),
                    "runtime": format!("{:?}", target.runtime),
                    "platform": format!("{:?}", target.platform),
                    "out_dir": target.out_dir.display().to_string(),
                    "out_file": target.out_file.as_ref().map(|path| path.display().to_string()),
                })
            })
            .collect::<Vec<_>>()
    });

    // derive workspace target summaries
    let workspace_targets = workspace_configs.as_ref().map(|configs| {
        configs
            .iter()
            .flat_map(|config| {
                config.options.targets.iter().map(|(name, target)| {
                    json!({
                        "name": name,
                        "output": format!("{:?}", target.output),
                        "runtime": format!("{:?}", target.runtime),
                        "platform": format!("{:?}", target.platform),
                        "out_dir": target.out_dir.display().to_string(),
                        "out_file": target.out_file.as_ref().map(|path| path.display().to_string()),
                        "package_dir": config.directory.display().to_string(),
                    })
                })
            })
            .collect::<Vec<_>>()
    });

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("info", 0);
        report.data = Some(json!({
            "workspace": {
                "root": workspace.root.display().to_string(),
                "kind": format!("{:?}", workspace.kind),
                "packages": package_paths,
            },
            "dsconfig": dsconfig_path.as_ref().map(|path| path.display().to_string()),
            "targets": targets,
            "workspace_targets": workspace_targets,
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    // emit minimal text output
    console::info(&format!("workspace: {}", workspace.root.display()));
    console::info(&format!("kind: {:?}", workspace.kind));
    for path in &package_paths {
        console::info(&format!("package: {path}"));
    }
    if let Some(path) = dsconfig_path {
        console::info(&format!("dsconfig: {}", path.display()));
    } else {
        console::warn("dsconfig: not found");
    }
    if let Some(targets) = targets {
        if targets.is_empty() {
            console::warn("targets: none");
        } else {
            for target in targets {
                if let Some(name) = target.get("name").and_then(|v| v.as_str()) {
                    console::info(&format!("target: {name}"));
                }
            }
        }
    }

    if let Some(workspace_targets) = workspace_targets
        && !workspace_targets.is_empty()
    {
        console::info(&format!("workspace targets: {}", workspace_targets.len()));
    }

    0
}
