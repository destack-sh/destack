use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report, report_error,
};
use crate::console;
use crate::pipeline::workspace::{
    load_dsconfig_for_program, load_workspace_dsconfigs, workspace_context,
};

/// Default cache directory name.
const DEFAULT_CACHE_DIR: &str = ".destack";

/// Arguments for the clean command.
#[derive(Args, Debug, Clone)]
pub struct CleanArgs {
    /// The directory to clean (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Remove all known Destack cache directories.
    #[arg(long)]
    pub all: bool,

    /// Show what would be removed without deleting.
    #[arg(long)]
    pub dry_run: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Remove build artifacts.
pub fn run(args: &CleanArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("clean", &args.program, &args.report) {
        return code;
    }

    // set up workspace context
    let context = match workspace_context(&args.program, args.dir.clone()) {
        Ok(context) => context,
        Err(message) => return report_error("clean", &args.report, &message),
    };
    let cwd = context.session.cwd.clone();

    // collect candidate paths for removal
    let mut paths = BTreeSet::new();

    // resolve dsconfigs based on scope
    let dsconfigs = if args.all {
        match load_workspace_dsconfigs(&context.resolver, &context.workspace) {
            Ok(dsconfigs) => dsconfigs,
            Err(message) => {
                return report_error(
                    "clean",
                    &args.report,
                    &format!("failed to load workspace configs: {message}"),
                );
            }
        }
    } else {
        match load_dsconfig_for_program(&args.program, &context.resolver, &cwd) {
            Ok(dsconfig) => vec![dsconfig],
            Err(message) => {
                return report_error("clean", &args.report, &message);
            }
        }
    };

    if dsconfigs.is_empty() {
        return report_error("clean", &args.report, "dsconfig.json not found");
    }

    // collect clean paths per config
    for dsconfig in &dsconfigs {
        collect_clean_paths(dsconfig, args.all, &mut paths);
    }

    // include workspace cache when requested
    if args.all {
        paths.insert(context.workspace.root.join(DEFAULT_CACHE_DIR));
    }

    // filter empty paths and bail when nothing is found
    let paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty())
        .collect();
    if paths.is_empty() {
        if args.report.is_json() {
            let mut report = CommandReport::success("clean", 0);
            report.summary = Some("no outputs to clean".to_string());
            print_report(&report, args.report.format());
        } else {
            console::info("clean: nothing to remove");
        }
        return 0;
    }

    // track removed paths and errors
    let mut removed = Vec::new();
    let mut errors = Vec::new();

    // remove entries or collect dry-run output
    for path in &paths {
        if args.dry_run {
            removed.push(path.display().to_string());
            continue;
        }

        if let Ok(metadata) = std::fs::metadata(path) {
            let result = if metadata.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
            match result {
                Ok(()) => removed.push(path.display().to_string()),
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
    }

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = if errors.is_empty() {
            CommandReport::success("clean", 0)
        } else {
            CommandReport::failure("clean", 1)
        };
        report.data = Some(json!({
            "removed": removed,
            "errors": errors,
            "dry_run": args.dry_run,
        }));
        print_report(&report, args.report.format());
    } else {
        for path in &removed {
            console::info(&format!("removed {path}"));
        }
        for error in &errors {
            console::error(error);
        }
    }

    // return failure when any errors are present
    if errors.is_empty() { 0 } else { 1 }
}

/// Collect known clean paths for a dsconfig.
fn collect_clean_paths(
    dsconfig: &destack_workspace::DsConfig,
    include_cache: bool,
    paths: &mut BTreeSet<PathBuf>,
) {
    // capture the package directory
    let package_dir = dsconfig.directory.clone();

    // compiler output directories
    if let Some(ref out_dir) = dsconfig.options.compiler.out_dir {
        paths.insert(resolve_path(out_dir, &package_dir));
    }
    if let Some(ref declaration_dir) = dsconfig.options.compiler.declaration_dir {
        paths.insert(resolve_path(declaration_dir, &package_dir));
    }

    // target output directories and files
    for (name, target_options) in &dsconfig.options.targets {
        let target = target_options.to_target(name);
        let out_dir = target.resolve_out_dir(&package_dir);
        paths.insert(out_dir);
        if let Some(ref out_file) = target.out_file {
            paths.insert(resolve_path(out_file, &package_dir));
        }
        if let Some(ref declaration_dir) = target.declaration_dir {
            paths.insert(resolve_path(declaration_dir, &package_dir));
        }
    }

    // include cache output when requested
    if include_cache {
        paths.insert(package_dir.join(DEFAULT_CACHE_DIR));
    }
}

/// Resolve a path relative to the workspace root.
fn resolve_path(path: &Path, root: &Path) -> PathBuf {
    // resolve relative paths against the root
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
