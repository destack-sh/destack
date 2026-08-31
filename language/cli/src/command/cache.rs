use std::path::Path;

use clap::Args;
use destack_artifact::{ArtifactCache, ArtifactCacheStats};
use serde::Serialize;

use crate::common::{
    ProgramArgs, ReportArgs, ensure_no_watch_or_dev, print_json_payload_report, report_error,
};
use crate::console;

/// Arguments for the cache command.
#[derive(Args, Debug, Clone)]
pub struct CacheArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Machine artifact cache report.
#[derive(Debug, Serialize)]
struct CachePayload<'a> {
    /// Machine cache directory.
    directory: &'a Path,
    /// Configured maximum cache size.
    maximum_bytes: Option<u64>,
    /// Exact cache usage.
    #[serde(flatten)]
    stats: ArtifactCacheStats,
}

/// Show machine artifact cache usage.
pub async fn run(args: &CacheArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("cache", &args.program, &args.report) {
        return code;
    }

    // resolve and measure the machine cache directly
    let (directory, maximum_bytes) = match args.program.resolve_artifact_cache() {
        Ok(cache) => cache,
        Err(error) => return report_error("cache", &args.report, &error.to_string()),
    };
    let stats = match ArtifactCache::measure(&directory, destack_workspace::Workspace::BUILD_ID) {
        Ok(stats) => stats,
        Err(error) => return report_error("cache", &args.report, &error.to_string()),
    };
    let payload = CachePayload {
        directory: &directory,
        maximum_bytes,
        stats,
    };

    // preserve exact values for machine-readable reports
    if let Err(code) = print_json_payload_report("cache", &args.report, 0, &payload) {
        return code;
    }
    if args.report.is_json() {
        return 0;
    }

    // render one human-readable cache report
    let size = match maximum_bytes {
        Some(maximum) => format!(
            "{} / {}",
            console::format_bytes(stats.bytes),
            console::format_bytes(maximum)
        ),
        None => console::format_bytes(stats.bytes),
    };
    let fields = [
        ("Directory", directory.display().to_string()),
        ("Size", size),
        (
            "Current build",
            console::format_bytes(stats.current_build_bytes),
        ),
        (
            "Other builds",
            console::format_bytes(stats.other_build_bytes),
        ),
        ("Builds", stats.builds.to_string()),
        ("Manifests", stats.manifests.to_string()),
        ("Packs", stats.packs.to_string()),
    ];
    console::print(&console::render_fields("Artifact cache", &fields));

    0
}
