use std::time::Duration;

use tspp_repository::{
    TraceSnapshot, TraceTimelineOptions, render_trace_duration, render_trace_timeline,
    trace_stage_color,
};

use super::console::{Stream, color_enabled, color_for_stream, format_duration, style_for_stream};

/// The drawn width of one timeline lane in cells.
const LANE_WIDTH: usize = 72;
/// The number of phase entries rendered per legend row.
const PHASES_PER_ROW: usize = 4;

/// Render artifact execution and its compact timing totals.
pub fn render_timings(report: &TraceSnapshot, command_duration: Option<Duration>) -> String {
    let work_micros = report.stages.iter().map(|stage| stage.micros).sum::<u64>();
    let phase_work = render_phase_work(report, work_micros);
    let timeline = render_timeline(report);
    let mut output = String::new();

    // render aggregate work by toolchain phase
    if !phase_work.is_empty() {
        output.push_str(&phase_work);
        output.push('\n');
    }

    // render artifact work across executor workers
    if !timeline.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("work by worker\n");
        output.push_str(&timeline);
        output.push('\n');
    }

    // collect the complete CLI command when its caller owns that clock
    let mut timings = Vec::new();
    if let Some(command_duration) = command_duration {
        let command = format_duration(command_duration);

        timings.push(format!("{command} command"));
    }

    // collect workspace wall time and summed parallel work
    let workspace = render_trace_duration(report.total_micros);
    timings.push(format!("{workspace} workspace"));
    if work_micros > 0 {
        let work = render_trace_duration(work_micros);

        timings.push(format!("{work} work"));
    }

    // collect concurrency and worker capacity
    if work_micros > 0 && report.total_micros > 0 && report.workers > 0 {
        let concurrency = work_micros as f64 / report.total_micros as f64;

        timings.push(format!("{concurrency:.2}× concurrency"));
        timings.push(format!("{} workers", report.workers));
    }

    // render the compact timing totals last
    output.push_str("timings · ");
    output.push_str(&timings.join(" · "));

    output.trim_end().to_string()
}

/// Render total artifact work as one proportional phase bar.
fn render_phase_work(report: &TraceSnapshot, total_micros: u64) -> String {
    if total_micros == 0 {
        return String::new();
    }

    // sample the midpoint of each cell against cumulative phase work
    let mut cells = Vec::with_capacity(LANE_WIDTH);
    let mut stage_index = 0;
    let mut stage_end = report.stages[0].micros;
    for cell in 0..LANE_WIDTH {
        let position =
            ((cell as u128 * 2 + 1) * total_micros as u128 / (LANE_WIDTH as u128 * 2)) as u64;
        while position >= stage_end && stage_index + 1 < report.stages.len() {
            stage_index += 1;
            stage_end += report.stages[stage_index].micros;
        }
        cells.push(stage_index);
    }

    // render adjacent cells of one phase as a single color run
    let mut bar = String::new();
    let mut first = 0;
    while first < cells.len() {
        let stage_index = cells[first];
        let mut end = first + 1;
        while end < cells.len() && cells[end] == stage_index {
            end += 1;
        }
        let blocks = "█".repeat(end - first);
        let stage = &report.stages[stage_index];
        let blocks = color_for_stream(&blocks, trace_stage_color(&stage.name), Stream::Stderr);
        bar.push_str(&blocks);
        first = end;
    }

    // render exact durations below the proportional bar
    let entries = report
        .stages
        .iter()
        .map(|stage| {
            let marker = color_for_stream("█", trace_stage_color(&stage.name), Stream::Stderr);
            let duration = render_trace_duration(stage.micros);

            format!("{marker} {} {duration}", stage.name)
        })
        .collect::<Vec<_>>();
    let separator = style_for_stream(" · ", &["2"], Stream::Stderr);
    let legend = entries
        .chunks(PHASES_PER_ROW)
        .map(|entries| format!("  {}", entries.join(&separator)))
        .collect::<Vec<_>>()
        .join("\n");
    let total = render_trace_duration(total_micros);

    format!("work by phase · {total}\n▕{bar}▏\n{legend}")
}

/// Render the per-worker timeline of one detailed build trace.
///
/// Each worker draws one lane, and every cell shows the artifact kind that owned most of its slice.
/// Color encodes its toolchain phase when available; plain output only marks busy and idle cells.
pub fn render_timeline(report: &TraceSnapshot) -> String {
    let options = TraceTimelineOptions::new()
        .with_color(color_enabled(Stream::Stderr))
        .with_width(LANE_WIDTH);

    render_trace_timeline(report, options)
}
