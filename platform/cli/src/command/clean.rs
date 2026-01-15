use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;

use destack_source::FileSystem;

use crate::common::{
    CommandError, CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_report,
    report_error,
};
use crate::console;
use crate::pipeline::cache::resolve_cache_location;
use crate::pipeline::workspace::{
    load_dsconfig_for_program, load_workspace_dsconfigs, workspace_context,
};

/// Arguments for the clean command.
#[derive(Args, Debug, Clone)]
pub struct CleanArgs {
    /// The directory to clean (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Remove build output directories.
    #[arg(long)]
    pub dist: bool,

    /// Remove cache directories.
    #[arg(long)]
    pub cache: bool,

    /// Remove all build outputs and caches.
    #[arg(long)]
    pub all: bool,

    /// Clean all packages in the workspace.
    #[arg(long = "all-packages", alias = "workspace-all")]
    pub all_packages: bool,

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

/// Remove build outputs and caches.
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
    let fs = context.session.fs.clone();

    // decide which outputs to clean
    let clean_dist = args.dist || args.all || !args.cache;
    let clean_cache = args.cache || args.all;

    // collect candidate paths for removal
    let mut paths = BTreeSet::new();

    // resolve dsconfigs based on scope
    let dsconfigs = if args.all_packages {
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
                if clean_dist {
                    return report_error("clean", &args.report, &message);
                }
                Vec::new()
            }
        }
    };

    if dsconfigs.is_empty() && clean_dist {
        return report_error("clean", &args.report, "dsconfig.json not found");
    }

    // collect clean paths per config
    for dsconfig in &dsconfigs {
        if clean_dist {
            collect_output_paths(dsconfig, &mut paths);
        }
        if clean_cache {
            let location = resolve_cache_location(
                &args.program,
                Some(dsconfig),
                &context.workspace.root,
                &cwd,
            );
            paths.insert(location.dir);
        }
    }

    // fall back to default cache path when no dsconfig is available
    if clean_cache && dsconfigs.is_empty() {
        let location = resolve_cache_location(&args.program, None, &context.workspace.root, &cwd);
        paths.insert(location.dir);
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

        match remove_path(fs.as_ref(), path) {
            Ok(true) => removed.push(path.display().to_string()),
            Ok(false) => {}
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = if errors.is_empty() {
            CommandReport::success("clean", 0)
        } else {
            let message = "clean encountered errors";
            let mut report = CommandReport::failure("clean", 1);
            report.summary = Some(message.to_string());
            report.error = Some(CommandError::new("clean_failed", "clean", message));
            report
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

/// Collect known output paths for a dsconfig.
fn collect_output_paths(dsconfig: &destack_workspace::DsConfig, paths: &mut BTreeSet<PathBuf>) {
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

/// Remove a file or directory recursively if it exists.
fn remove_path(fs: &dyn FileSystem, path: &Path) -> io::Result<bool> {
    // fetch metadata without following symlinks
    let metadata = match fs.symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };

    // remove symlinks and files directly
    if metadata.is_symlink || metadata.is_file {
        fs.remove_file(path)?;
        return Ok(true);
    }

    // remove directories recursively
    if metadata.is_directory {
        remove_dir_all(fs, path)?;
        return Ok(true);
    }

    Ok(false)
}

/// Remove a directory and all of its entries.
fn remove_dir_all(fs: &dyn FileSystem, dir: &Path) -> io::Result<()> {
    // collect directory entries
    let entries = fs.read_dir(dir)?;

    // remove child entries
    for entry in entries {
        remove_path(fs, &entry)?;
    }

    // remove the directory itself
    fs.remove_dir(dir)
}
