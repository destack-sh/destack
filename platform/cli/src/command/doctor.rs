use std::process::Command;

use clap::Args;
use serde::Serialize;

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report, report_error,
};
use crate::console;
use crate::pipeline::workspace::{load_dsconfig, resolve_dsconfig_path, workspace_context};

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    // collect static environment info
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let available_parallelism = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);

    let context = match workspace_context(&args.program, None) {
        Ok(context) => context,
        Err(message) => return report_error("doctor", &args.report, &message),
    };
    let cwd = context.session.cwd.clone();

    let dsconfig_path = match resolve_dsconfig_path(&args.program, &context.resolver, &cwd) {
        Ok(path) => Some(path),
        Err(message) => {
            if args.program.config.is_some() {
                return report_error("doctor", &args.report, &message);
            }
            None
        }
    };

    let dsconfig = dsconfig_path
        .as_ref()
        .and_then(|path| load_dsconfig(&context.resolver, path).ok());

    // collect dsconfig warnings
    let mut warnings = Vec::new();
    if dsconfig_path.is_none() {
        warnings.push("dsconfig.json not found".to_string());
    }

    let mut target_names = Vec::new();
    let mut default_target = None;
    let mut extends = Vec::new();
    if let Some(config) = dsconfig.as_ref() {
        target_names = config.options.targets.keys().cloned().collect();
        default_target = config.options.default_target.clone();
        extends = list_extends(config);
        if let Some(default_target) = default_target.as_ref()
            && !config.options.targets.contains_key(default_target)
        {
            warnings.push(format!("default target '{default_target}' is not defined"));
        }
        if config.options.targets.is_empty() {
            warnings.push("no targets configured".to_string());
        }
    }

    // collect toolchain info only when requested
    let tools = if args.full {
        vec![
            probe_tool("rustc", &["--version"]),
            probe_tool("cargo", &["--version"]),
            probe_tool("bun", &["--version"]),
            probe_tool("node", &["--version"]),
        ]
    } else {
        Vec::new()
    };

    let package_paths: Vec<String> = context
        .workspace
        .package_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    if args.report.is_json() {
        let mut report = CommandReport::success("doctor", 0);
        report.data = Some(serde_json::json!({
            "cli_version": CLI_VERSION,
            "cwd": cwd.display().to_string(),
            "os": os,
            "arch": arch,
            "workers": args.program.workers,
            "available_parallelism": available_parallelism,
            "workspace": {
                "root": context.workspace.root.display().to_string(),
                "kind": format!("{:?}", context.workspace.kind),
                "package_count": package_paths.len(),
                "packages": if args.full { Some(package_paths) } else { None },
            },
            "dsconfig": dsconfig_path.as_ref().map(|path| path.display().to_string()),
            "extends": if extends.is_empty() { None } else { Some(extends) },
            "default_target": default_target,
            "target_count": target_names.len(),
            "targets": if args.full { Some(target_names) } else { None },
            "tools": if args.full { Some(tools) } else { None },
            "warnings": if warnings.is_empty() { None } else { Some(warnings) },
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    console::info(&format!("destack {CLI_VERSION}"));
    console::info(&format!("cwd: {}", cwd.display()));
    console::info(&format!("os: {os}"));
    console::info(&format!("arch: {arch}"));
    console::info(&format!("workers: {}", args.program.workers));
    console::info(&format!("available parallelism: {available_parallelism}"));
    console::info(&format!(
        "workspace: {} ({:?})",
        context.workspace.root.display(),
        context.workspace.kind
    ));
    console::info(&format!("packages: {}", package_paths.len()));
    if args.full {
        for package in &package_paths {
            console::info(&format!("package: {package}"));
        }
    }
    if let Some(path) = dsconfig_path {
        console::info(&format!("dsconfig: {}", path.display()));
    } else {
        console::warn("dsconfig: not found");
    }
    if let Some(default_target) = default_target.as_ref() {
        console::info(&format!("default target: {default_target}"));
    }
    if !extends.is_empty() {
        console::info(&format!("extends: {}", extends.join(", ")));
    }
    console::info(&format!("targets: {}", target_names.len()));
    if args.full {
        for name in &target_names {
            console::info(&format!("target: {name}"));
        }
    }

    if args.full && !tools.is_empty() {
        console::info("tools:");
        for tool in &tools {
            match tool.status {
                ToolStatus::Available => {
                    if let Some(version) = tool.version.as_ref() {
                        console::info(&format!("  {}: {version}", tool.name));
                    } else {
                        console::info(&format!("  {}: available", tool.name));
                    }
                }
                ToolStatus::Missing => {
                    console::warn(&format!("  {}: not found", tool.name));
                }
                ToolStatus::Error => {
                    console::warn(&format!("  {}: error", tool.name));
                }
            }
        }
    }

    for warning in &warnings {
        console::warn(&format!("warning: {warning}"));
    }

    0
}

/// Tool probing result for doctor output.
#[derive(Debug, Serialize)]
struct ToolInfo {
    /// Tool name.
    name: &'static str,
    /// Tool version string.
    version: Option<String>,
    /// Tool probe status.
    status: ToolStatus,
}

/// Status values for tool detection.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ToolStatus {
    /// Tool responded successfully.
    Available,
    /// Tool was not found on the system.
    Missing,
    /// Tool returned an error response.
    Error,
}

/// Query a tool for its version string.
fn probe_tool(name: &'static str, args: &[&str]) -> ToolInfo {
    let output = Command::new(name).args(args).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let version = parse_version_output(&output.stdout, &output.stderr);
                ToolInfo {
                    name,
                    version,
                    status: ToolStatus::Available,
                }
            } else {
                ToolInfo {
                    name,
                    version: None,
                    status: ToolStatus::Error,
                }
            }
        }
        Err(error) => {
            let status = if error.kind() == std::io::ErrorKind::NotFound {
                ToolStatus::Missing
            } else {
                ToolStatus::Error
            };
            ToolInfo {
                name,
                version: None,
                status,
            }
        }
    }
}

/// Parse version text from tool stdout/stderr output.
fn parse_version_output(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let stdout = stdout.trim();
    if !stdout.is_empty() {
        return Some(stdout.to_string());
    }
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        return Some(stderr.to_string());
    }
    None
}

/// Collect the dsconfig extends entries.
fn list_extends(dsconfig: &destack_workspace::DsConfig) -> Vec<String> {
    match dsconfig.content.extends.as_ref() {
        Some(destack_workspace::ExtendsFieldJson::Single(value)) => vec![value.clone()],
        Some(destack_workspace::ExtendsFieldJson::Multiple(values)) => values.clone(),
        None => Vec::new(),
    }
}
