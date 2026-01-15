use std::path::PathBuf;

use clap::Args;
use serde::Serialize;

use crate::common::{
    CommandReport, ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs,
    ensure_no_watch_or_dev, list_payload, print_list_with, print_report, report_error,
};
use crate::console;
use crate::pipeline::cache::{CacheSource, resolve_cache_location};
use crate::pipeline::workspace::{
    load_dsconfig_for_program, load_workspace_dsconfigs, workspace_context,
};

/// Arguments for the cache command.
#[derive(Args, Debug, Clone)]
pub struct CacheArgs {
    /// Show cache locations for all workspace packages.
    #[arg(long = "all-packages", alias = "workspace-all")]
    pub all_packages: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Cache entry returned by the cache command.
#[derive(Serialize)]
struct CacheEntry {
    /// Cache directory path.
    dir: PathBuf,
    /// Source of the cache location.
    source: &'static str,
    /// Package directory when available.
    package_dir: Option<PathBuf>,
}

/// Show cache directory locations.
pub fn run(args: &CacheArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("cache", &args.program, &args.report) {
        return code;
    }

    // resolve workspace context
    let context = match workspace_context(&args.program, None) {
        Ok(context) => context,
        Err(message) => {
            return report_error("cache", &args.report, &message);
        }
    };
    let cwd = context.session.cwd.clone();

    // load dsconfigs based on scope
    let dsconfigs = if args.all_packages {
        match load_workspace_dsconfigs(&context.resolver, &context.workspace) {
            Ok(dsconfigs) => dsconfigs,
            Err(message) => {
                return report_error(
                    "cache",
                    &args.report,
                    &format!("failed to load workspace configs: {message}"),
                );
            }
        }
    } else {
        match load_dsconfig_for_program(&args.program, &context.resolver, &cwd) {
            Ok(dsconfig) => vec![dsconfig],
            Err(message) => {
                return report_error("cache", &args.report, &message);
            }
        }
    };

    // collect cache locations
    let mut entries = Vec::new();
    if dsconfigs.is_empty() {
        let location = resolve_cache_location(&args.program, None, &context.workspace.root, &cwd);
        entries.push(CacheEntry {
            dir: location.dir,
            source: cache_source_label(location.source),
            package_dir: None,
        });
    } else {
        for dsconfig in dsconfigs {
            let location = resolve_cache_location(
                &args.program,
                Some(&dsconfig),
                &context.workspace.root,
                &cwd,
            );
            entries.push(CacheEntry {
                dir: location.dir,
                source: cache_source_label(location.source),
                package_dir: Some(dsconfig.directory),
            });
        }
    }

    if args.report.is_json() {
        let mut report = CommandReport::success("cache", 0);
        report.data = Some(list_payload(entries));
        print_report(&report, args.report.format());
        return 0;
    }

    let list_entries = entries.into_iter().map(|entry| {
        let location = entry.dir.display();
        let source = entry.source;
        let title = if let Some(package_dir) = entry.package_dir {
            format!("{location} ({source}, package: {})", package_dir.display())
        } else {
            format!("{location} ({source})")
        };
        ListEntry::new(title)
    });
    let list_entries: Vec<ListEntry> = list_entries.collect();
    if list_entries.is_empty() {
        console::info("cache: no entries");
        return 0;
    }
    let printer = ListPrinter::info();
    print_list_with(&list_entries, ListSpacing::Compact, &printer);

    0
}

/// Render a label for the cache source.
fn cache_source_label(source: CacheSource) -> &'static str {
    match source {
        CacheSource::Override => "override",
        CacheSource::DsConfig => "dsconfig",
        CacheSource::Default => "default",
    }
}
