use destack_repository::TraceReport;

use super::console::{Stream, bold, color, color_enabled, dim};

/// The drawn width of one timeline lane in cells.
const LANE_WIDTH: usize = 72;

/// The number of slowest artifacts listed under the timeline.
const SLOWEST_COUNT: usize = 8;

/// Render the one-line stage summary of one build trace.
///
/// Example:
/// ```text
/// parse 450ms · sema 6.1s · lower 320ms · emit 95ms (wall 2.1s, 8 workers)
/// ```
pub fn render_stage_summary(report: &TraceReport) -> String {
    // a run without provider work was served from cache
    if report.stages.is_empty() {
        return dim("all artifacts cached").to_string();
    }

    let colored = color_enabled(Stream::Stdout);
    let stages = report
        .stages
        .iter()
        .map(|stage| {
            let entry = format!("{} {}", stage.name, render_duration(stage.micros));
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
            render_duration(report.total_micros),
            report.workers,
            if report.workers == 1 {
                "worker"
            } else {
                "workers"
            }
        ))
    )
}

/// One artifact kind drawn on a timeline.
struct TimelineKind {
    /// The artifact kind name.
    name: String,
    /// The lane glyph index of the kind's stage.
    stage: usize,
    /// The summed busy time across the run.
    micros: u64,
}

/// Render the per-worker timeline of one detailed build trace.
/// Each worker draws one lane; every cell shows the artifact kind that
/// owned most of its slice of wall time. The glyph encodes the stage,
/// the color encodes the artifact kind.
pub fn render_timeline(report: &TraceReport) -> String {
    if report.artifacts.is_empty() || report.total_micros == 0 {
        return String::new();
    }

    let mut output = String::new();
    let cell_micros = report.total_micros.div_ceil(LANE_WIDTH as u64).max(1);
    let colored = color_enabled(Stream::Stdout);

    // index the artifact kinds appearing in this trace
    let mut kinds: Vec<TimelineKind> = Vec::new();
    for artifact in &report.artifacts {
        match kinds.iter_mut().find(|kind| kind.name == artifact.name) {
            Some(kind) => kind.micros += artifact.micros,
            None => kinds.push(TimelineKind {
                name: artifact.name.clone(),
                stage: stage_index(&artifact.stage),
                micros: artifact.micros,
            }),
        }
    }

    // one lane per worker, busiest kind per cell
    for worker in 0..report.workers {
        let mut busy = vec![vec![0u64; kinds.len()]; LANE_WIDTH];
        for artifact in report.artifacts.iter().filter(|a| a.worker == worker) {
            let kind = kinds
                .iter()
                .position(|kind| kind.name == artifact.name)
                .expect("every drawn artifact kind is indexed");
            let end = artifact.start_micros + artifact.micros.max(1);
            let first = (artifact.start_micros / cell_micros) as usize;
            let last = ((end - 1) / cell_micros) as usize;
            for cell in first..=last.min(LANE_WIDTH - 1) {
                let cell_start = cell as u64 * cell_micros;
                let cell_end = cell_start + cell_micros;
                let overlap = end
                    .min(cell_end)
                    .saturating_sub(artifact.start_micros.max(cell_start));
                busy[cell][kind] += overlap;
            }
        }

        // pick each cell's winner, then paint coalesced runs
        let cells = busy
            .into_iter()
            .map(|cell| {
                cell.iter()
                    .enumerate()
                    .filter(|(_, micros)| **micros > 0)
                    .max_by_key(|(_, micros)| **micros)
                    .map(|(kind, _)| kind)
            })
            .collect::<Vec<_>>();
        let mut lane = String::new();
        let mut run = String::new();
        let mut run_kind: Option<usize> = None;
        for cell in cells {
            if cell != run_kind && !run.is_empty() {
                lane.push_str(&paint_run(&run, run_kind, &kinds, colored));
                run.clear();
            }
            run_kind = cell;
            run.push(match cell {
                Some(_) if colored => '█',
                Some(kind) => STAGE_CELLS[kinds[kind].stage],
                None => '·',
            });
        }
        lane.push_str(&paint_run(&run, run_kind, &kinds, colored));
        output.push_str(&format!("worker {worker:>2} ▕{lane}▏\n"));
    }

    // time scale
    output.push_str(&dim(&format!(
        "          0{:>width$}\n",
        render_duration(report.total_micros),
        width = LANE_WIDTH,
    )));

    // legend: colored kinds when possible, stage glyphs otherwise
    if colored {
        let mut legend = kinds.iter().collect::<Vec<_>>();
        legend.sort_by(|left, right| right.micros.cmp(&left.micros));
        let entries = legend
            .into_iter()
            .map(|kind| format!("{} {}", color("█", kind_color(&kind.name)), kind.name))
            .collect::<Vec<_>>();
        for line in entries.chunks(4) {
            output.push_str(&format!("          {}\n", line.join("  ")));
        }
    } else {
        output.push_str(&dim(
            "          ░ parse  ▒ bind  ▚ macro  ▓ check  ▆ lower  █ emit  ▄ link  ▁ other  · idle\n",
        ));
    }

    // the slowest ready artifacts carry the useful names
    let mut slowest = report
        .artifacts
        .iter()
        .filter(|artifact| artifact.outcome == "ready")
        .collect::<Vec<_>>();
    slowest.sort_by(|left, right| right.micros.cmp(&left.micros));
    if !slowest.is_empty() {
        output.push_str(&format!("\n{}\n", bold("slowest artifacts:")));
        for artifact in slowest.into_iter().take(SLOWEST_COUNT) {
            let label = artifact.label.as_deref().unwrap_or("");
            let name = if colored {
                color(
                    &format!("{:<24}", artifact.name),
                    kind_color(&artifact.name),
                )
            } else {
                format!("{:<24}", artifact.name)
            };
            let target = artifact
                .target
                .as_deref()
                .map(|target| dim(&format!(" ({target})")))
                .unwrap_or_default();
            output.push_str(&format!(
                "  {:>9}  {name} {label}{target}\n",
                render_duration(artifact.micros),
            ));
        }
    }

    output
}

/// Paint one coalesced run of equal lane cells.
fn paint_run(run: &str, kind: Option<usize>, kinds: &[TimelineKind], colored: bool) -> String {
    match kind {
        Some(kind) if colored => color(run, kind_color(&kinds[kind].name)),
        Some(_) => run.to_string(),
        None => dim(run),
    }
}

/// The lane glyph drawn for each stage, ordered like stage_index.
/// Lint, query, and setup share one glyph; color and the legend carry
/// the kind.
const STAGE_CELLS: [char; 8] = ['░', '▒', '▚', '▓', '▆', '█', '▄', '▁'];

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
        "check" => "38;5;170",
        "lower" => "38;5;208",
        "link" => "38;5;84",
        "lint" => "38;5;228",
        "query" => "38;5;147",
        "init" => "38;5;245",
        _ => "38;5;250",
    }
}

/// Return the lane glyph index of one stage display name.
fn stage_index(stage: &str) -> usize {
    match stage {
        "parse" => 0,
        "bind" => 1,
        "macro" => 2,
        "check" => 3,
        "lower" => 4,
        "emit" => 5,
        "link" => 6,
        _ => 7,
    }
}

/// Return the 256-color code of one artifact kind display name.
fn kind_color(name: &str) -> &'static str {
    match name {
        "dir.parse" => "38;5;75",
        "data" => "38;5;67",
        "dir.bind" => "38;5;80",
        "dir.import" => "38;5;73",
        "dir.expand" => "38;5;115",
        "dir.export" => "38;5;72",
        "dir.resolve" => "38;5;79",
        "dir.check.component" => "38;5;170",
        "dir.check" => "38;5;176",
        "dir.materialize" => "38;5;178",
        "dir.elaborate" => "38;5;179",
        "mir.lower" => "38;5;208",
        "mir.verify" => "38;5;209",
        "mir.optimize" => "38;5;214",
        "module.emit" => "38;5;114",
        "package.link" => "38;5;84",
        "module.lint" | "package.lint" | "workspace.lint" => "38;5;228",
        "module.index" | "workspace.index" => "38;5;147",
        "environment" | "dependency.index" => "38;5;245",
        _ => "38;5;250",
    }
}

/// Render one microsecond count as a compact duration.
fn render_duration(micros: u64) -> String {
    if micros >= 10_000_000 {
        format!("{:.1}s", micros as f64 / 1_000_000.0)
    } else if micros >= 1_000_000 {
        format!("{:.2}s", micros as f64 / 1_000_000.0)
    } else if micros >= 1_000 {
        format!("{}ms", micros / 1_000)
    } else {
        format!("{micros}µs")
    }
}
