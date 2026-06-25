use destack_repository::{
    TraceSnapshot, TraceTimelineOptions, render_trace_duration, render_trace_timeline,
};

use super::console::{Stream, color, color_enabled, dim};

/// The drawn width of one timeline lane in cells.
const LANE_WIDTH: usize = 72;

/// The number of slowest artifacts listed under the timeline.
const SLOWEST_COUNT: usize = 8;

/// Render the one-line stage summary of one build trace.
///
/// Example:
/// ```text
/// parse 450ms · check 6.1s · lower 320ms · emit 95ms (wall 2.1s, 8 workers)
/// ```
pub fn render_stage_summary(report: &TraceSnapshot) -> String {
    // a run without provider work was served from cache
    if report.stages.is_empty() {
        return dim("all artifacts cached").to_string();
    }

    let colored = color_enabled(Stream::Stdout);
    let stages = report
        .stages
        .iter()
        .map(|stage| {
            let entry = format!("{} {}", stage.name, render_trace_duration(stage.micros));
            if colored {
                color(&entry, stage_color(&stage.name))
            } else {
                entry
            }
        })
        .collect::<Vec<_>>()
        .join(&dim(" · "));

    format!(
        "{stages} {}",
        dim(&format!(
            "(wall {}, {} {})",
            render_trace_duration(report.total_micros),
            report.workers,
            if report.workers == 1 {
                "worker"
            } else {
                "workers"
            }
        ))
    )
}

/// Render the per-worker timeline of one detailed build trace.
/// Each worker draws one lane; every cell shows the artifact kind that
/// owned most of its slice of wall time. Color encodes artifact kind
/// when available; plain output only marks busy and idle cells.
pub fn render_timeline(report: &TraceSnapshot) -> String {
    let options = TraceTimelineOptions::new()
        .with_color(color_enabled(Stream::Stdout))
        .with_width(LANE_WIDTH)
        .with_slow_attempts(SLOWEST_COUNT);

    render_trace_timeline(report, options)
}

/// Return the 256-color code of one stage display name, matching the
/// lead artifact kind drawn in the timeline.
fn stage_color(stage: &str) -> &'static str {
    if stage.starts_with("emit") {
        return "38;5;114";
    }

    match stage {
        "parse" => "38;5;75",
        "bind" => "38;5;80",
        "macro" => "38;5;115",
        "resolve" => "38;5;79",
        "graph" => "38;5;147",
        "check" => "38;5;170",
        "lower" => "38;5;208",
        "link" => "38;5;84",
        "lint" => "38;5;228",
        "query" => "38;5;147",
        "init" => "38;5;245",
        _ => "38;5;250",
    }
}
