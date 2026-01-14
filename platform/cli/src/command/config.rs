use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::Value;

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report, report_error,
};
use crate::console;
use crate::pipeline::workspace::{find_dsconfig, load_dsconfig, workspace_context};

/// Arguments for the config command.
#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    /// The config file or directory to inspect.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Show full config content in text mode.
    #[arg(long)]
    pub full: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show the resolved configuration.
pub fn run(args: &ConfigArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("config", &args.program, &args.report) {
        return code;
    }

    // set up workspace context
    let context = match workspace_context(&args.program, args.path.clone()) {
        Ok(context) => context,
        Err(message) => return report_error("config", &args.report, &message),
    };
    let cwd = context.session.cwd.clone();

    // resolve dsconfig path
    let config_path = args.path.as_ref().or(args.program.config.as_ref());
    let dsconfig_path = match resolve_config_path(&context.resolver, config_path, &cwd) {
        Ok(path) => path,
        Err(message) => return report_error("config", &args.report, &message),
    };

    // load dsconfig for summary fields
    let dsconfig = match load_dsconfig(&context.resolver, &dsconfig_path) {
        Ok(dsconfig) => dsconfig,
        Err(message) => return report_error("config", &args.report, &message),
    };

    // parse raw config json
    let config_json = match read_config_json(&dsconfig_path) {
        Ok(value) => value,
        Err(message) => return report_error("config", &args.report, &message),
    };

    // collect target metadata
    let target_names: Vec<String> = dsconfig.options.targets.keys().cloned().collect();
    let default_target = dsconfig.options.default_target.clone();

    // print structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("config", 0);
        report.data = Some(serde_json::json!({
            "path": dsconfig_path.display().to_string(),
            "default_target": default_target,
            "targets": target_names,
            "config": config_json,
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    // emit minimal text output
    console::info(&format!("config: {}", dsconfig_path.display()));
    if let Some(default_target) = default_target {
        console::info(&format!("default target: {default_target}"));
    }
    if target_names.is_empty() {
        console::info("targets: none");
    } else {
        console::info(&format!("targets: {}", target_names.join(", ")));
    }

    if args.full {
        // pretty print the full config
        if let Ok(pretty) = serde_json::to_string_pretty(&config_json) {
            println!("{pretty}");
        }
    }

    0
}

/// Resolve the config path from command arguments.
fn resolve_config_path(
    resolver: &destack_resolver::Resolver,
    path: Option<&PathBuf>,
    cwd: &Path,
) -> Result<PathBuf, String> {
    // honor explicit paths when provided
    if let Some(path) = path {
        let resolved_path = if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        };

        if resolved_path.is_file() {
            return Ok(resolved_path);
        }
        if resolved_path.is_dir() {
            return find_dsconfig(resolver, &resolved_path)
                .ok_or_else(|| "dsconfig.json not found".to_string());
        }
        return Err(format!("path not found: {}", resolved_path.display()));
    }

    // fall back to cwd lookup
    find_dsconfig(resolver, cwd).ok_or_else(|| "dsconfig.json not found".to_string())
}

/// Read and parse a dsconfig.json file into JSON.
fn read_config_json(path: &PathBuf) -> Result<Value, String> {
    // read the config file contents
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    // parse into json value
    serde_json::from_str(&content).map_err(|e| format!("invalid dsconfig: {e}"))
}
