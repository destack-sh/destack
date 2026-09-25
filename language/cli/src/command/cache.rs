use std::path::Path;

use clap::Args;
use serde::Serialize;
use tspp_artifact::{ArtifactCache, ArtifactCacheStats, ArtifactCacheUsage};

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
struct CacheReport<'a> {
    /// Machine cache directory.
    directory: &'a Path,
    /// Configured maximum cache size.
    maximum_bytes: Option<u64>,
    /// Exact cache usage.
    #[serde(flatten)]
    usage: ArtifactCacheUsage,
}

impl CacheReport<'_> {
    /// Render this report for a human terminal.
    fn render(&self) -> String {
        let used = console::format_bytes(self.usage.total.bytes);
        let mut output = format!(
            "{}\n{}\n\n",
            console::bold("Artifact cache"),
            console::dim(&self.directory.display().to_string())
        );

        // render bounded usage when a maximum is configured
        if let Some(maximum) = self.maximum_bytes {
            let bar = console::UsageBar::new(self.usage.total.bytes, maximum);
            let maximum = console::format_bytes(maximum);
            let percentage = match bar.percentage() {
                Some(percentage) => format!("{percentage:.1}%"),
                None if self.usage.total.bytes == 0 => "0.0%".to_string(),
                None => "over limit".to_string(),
            };
            output.push_str(&format!(
                "{} of {maximum}  {}\n{}\n\n",
                console::bold(&used),
                console::dim(&percentage),
                bar.render()
            ));
        } else {
            output.push_str(&format!(
                "{} used  {}\n\n",
                console::bold(&used),
                console::dim("unbounded")
            ));
        }

        // render the exact cache records by build scope
        let current = Self::row("Current build", self.usage.current_build);
        let other = Self::row("Other builds", self.usage.other_builds);
        let total = Self::row("Total", self.usage.total).map(|cell| console::bold(&cell));
        let rows = [current, other, total];

        // align labels and exact numeric values explicitly
        let columns = [
            console::TableColumn::left("Scope"),
            console::TableColumn::right("Builds"),
            console::TableColumn::right("Files"),
            console::TableColumn::right("Manifests"),
            console::TableColumn::right("Packs"),
            console::TableColumn::right("Size"),
        ];

        // append the complete table below the usage bar
        let table = console::Table::new(columns, &rows).render();
        output.push_str(table.trim_end());

        output
    }

    /// Render one cache usage row.
    fn row(label: &str, stats: ArtifactCacheStats) -> [String; 6] {
        [
            label.to_string(),
            stats.builds.to_string(),
            stats.files.to_string(),
            stats.manifests.to_string(),
            stats.packs.to_string(),
            console::format_bytes(stats.bytes),
        ]
    }
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
    let usage = match ArtifactCache::measure(&directory, tspp_workspace::Workspace::BUILD_ID) {
        Ok(usage) => usage,
        Err(error) => return report_error("cache", &args.report, &error.to_string()),
    };
    let report = CacheReport {
        directory: &directory,
        maximum_bytes,
        usage,
    };

    // preserve exact values for machine-readable reports
    if let Err(code) = print_json_payload_report("cache", &args.report, 0, &report) {
        return code;
    }
    if args.report.is_json() {
        return 0;
    }

    // render one human-readable cache report
    console::print(&report.render());

    0
}
