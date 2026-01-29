use std::collections::HashMap;
use std::time::Duration;

use clap::{Args, ValueEnum};
use destack_compiler::StatsSnapshot;
use destack_daemon::protocol::{BinaryPayload, CommandMessagePayload};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::common::compile::print_no_input_help;
use crate::common::format::{DiagnosticOutputJson, LineWriter};
use crate::console;

/// Schema version for command reports.
pub const REPORT_SCHEMA_VERSION: u32 = 4;

/// Output format for command reports.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ReportFormat {
    /// Human-readable text output.
    #[default]
    Text,
    /// JSON output for tooling integration.
    Json,
}

impl ReportFormat {
    /// Whether this report format is JSON.
    pub fn is_json(self) -> bool {
        // check for json enum variant
        matches!(self, Self::Json)
    }
}

/// Common report formatting arguments for commands.
#[derive(Args, Debug, Clone, Default)]
pub struct ReportArgs {
    /// Output format for the command (text or json).
    #[arg(long = "output-format", value_enum)]
    pub output_format: Option<ReportFormat>,

    /// Emit JSON output (shorthand for --output-format json).
    #[arg(long)]
    pub json: bool,

    /// Maximum timing tags to display in text output.
    #[arg(long = "timings-top", default_value_t = 15)]
    pub timings_top: usize,

    /// Minimum timing tag duration in milliseconds.
    #[arg(long = "timings-min-ms", default_value_t = 1)]
    pub timings_min_ms: u64,
}

impl ReportArgs {
    /// Resolve the requested report format.
    pub fn format(&self) -> ReportFormat {
        // honor the json flag first
        if self.json {
            return ReportFormat::Json;
        }

        // fall back to the explicit format or text
        self.output_format.unwrap_or(ReportFormat::Text)
    }

    /// Whether JSON output was requested.
    pub fn is_json(&self) -> bool {
        // compare the resolved format
        matches!(self.format(), ReportFormat::Json)
    }
}

/// Result status for a command report.
#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    /// Command completed successfully.
    Success,
    /// Command failed.
    Failure,
}

/// Summary statistics for command execution.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandStats {
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// Number of tasks completed by the compiler.
    pub tasks_completed: usize,
    /// Number of tasks failed by the compiler.
    pub tasks_failed: usize,
    /// Number of tasks skipped by the compiler.
    pub tasks_skipped: usize,
    /// Number of modules processed.
    pub modules_processed: usize,
    /// Number of lines processed.
    pub lines_processed: usize,
    /// Number of slow tasks detected.
    pub slow_tasks: usize,
    /// Cache statistics for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CommandCacheStats>,
    /// Timing tag statistics for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<Vec<CommandTimingTagStats>>,
}

impl CommandStats {
    /// Build stats from a compiler snapshot.
    pub fn from_snapshot(snapshot: &StatsSnapshot) -> Self {
        // map snapshot fields into the report summary
        let elapsed_ms = duration_to_ms(snapshot.elapsed);
        let modules_processed = snapshot.modules_processed();
        let cache = cache_stats_from_snapshot(snapshot);
        Self {
            elapsed_ms,
            tasks_completed: snapshot.tasks.completed,
            tasks_failed: snapshot.tasks.failed,
            tasks_skipped: snapshot.tasks.skipped,
            modules_processed,
            lines_processed: snapshot.modules.lines_processed,
            slow_tasks: snapshot.slow_tasks,
            cache,
            timings: None,
        }
    }
}

/// Timing tag statistics for command output.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandTimingTagStats {
    /// Timing tag name.
    pub name: String,
    /// Total time spent in this tag (milliseconds).
    pub duration_ms: u64,
    /// Number of samples recorded.
    pub sample_count: u64,
}

/// Options for reporting timing tags.
#[derive(Debug, Clone, Copy)]
pub struct TimingOutputOptions {
    /// Whether timing output is enabled.
    pub enabled: bool,
    /// Maximum number of entries to display.
    pub top: usize,
    /// Minimum duration in milliseconds to display.
    pub min_ms: u64,
}

/// Cache statistics for command output.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandCacheStats {
    /// Cache hits from memory.
    pub hits_memory: usize,
    /// Cache hits from disk.
    pub hits_disk: usize,
    /// Cache misses.
    pub misses: usize,
    /// Cache writes to memory.
    pub writes_memory: usize,
    /// Cache writes to disk.
    pub writes_disk: usize,
    /// Cache errors.
    pub errors: usize,
    /// Cache hit rate across all cache kinds.
    pub hit_rate: f32,
}

/// Build cache stats from a compiler snapshot when there is cache activity.
fn cache_stats_from_snapshot(snapshot: &StatsSnapshot) -> Option<CommandCacheStats> {
    let totals = snapshot.cache_totals();
    let activity = totals.hits_memory
        + totals.hits_disk
        + totals.misses
        + totals.writes_memory
        + totals.writes_disk
        + totals.errors;

    if activity == 0 {
        return None;
    }

    Some(CommandCacheStats {
        hits_memory: totals.hits_memory,
        hits_disk: totals.hits_disk,
        misses: totals.misses,
        writes_memory: totals.writes_memory,
        writes_disk: totals.writes_disk,
        errors: totals.errors,
        hit_rate: snapshot.cache_hit_rate(),
    })
}

/// Structured error payload for command failures.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandError {
    /// Machine-readable error code.
    pub code: String,
    /// Error category for tooling.
    pub kind: String,
    /// Human-readable error message.
    pub message: String,
}

impl CommandError {
    /// Build a structured command error.
    pub fn new(
        code: impl Into<String>,
        kind: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            kind: kind.into(),
            message: message.into(),
        }
    }
}

/// Report output for a command invocation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Command name.
    pub command: String,
    /// Result status.
    pub status: CommandStatus,
    /// Exit code returned by the command.
    pub exit_code: i32,
    /// Short summary for text output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Diagnostics payload for tooling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticOutputJson>,
    /// Structured error payload when diagnostics are not available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandError>,
    /// Compiler stats snapshot for tooling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<CommandStats>,
    /// Command-specific payload.
    #[cfg_attr(feature = "schema", schemars(schema_with = "schema_any"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl CommandReport {
    /// Build a success report with the given exit code.
    pub fn success(command: &str, exit_code: i32) -> Self {
        // create a report with success status
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            command: command.to_string(),
            status: CommandStatus::Success,
            exit_code,
            summary: None,
            diagnostics: None,
            error: None,
            stats: None,
            data: None,
        }
    }

    /// Build a failure report with the given exit code.
    pub fn failure(command: &str, exit_code: i32) -> Self {
        // create a report with failure status
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            command: command.to_string(),
            status: CommandStatus::Failure,
            exit_code,
            summary: None,
            diagnostics: None,
            error: None,
            stats: None,
            data: None,
        }
    }
}

/// Print a command report in the requested format.
pub fn print_report(report: &CommandReport, format: ReportFormat) {
    // format and emit the report payload
    match format {
        ReportFormat::Text => {
            if let Some(summary) = report.summary.as_ref() {
                match report.status {
                    CommandStatus::Success => console::success(summary),
                    CommandStatus::Failure => console::error(summary),
                }
            }
        }
        ReportFormat::Json => {
            // serialize as pretty json
            if let Ok(json) = serde_json::to_string_pretty(report) {
                println!("{json}");
            }
        }
    }
}

/// Report a command error using text or JSON output.
pub fn report_error(command: &str, report_args: &ReportArgs, message: &str) -> i32 {
    report_error_with(
        command,
        report_args,
        CommandError::new("cli_error", "cli", message),
    )
}

/// Report a command error with structured error details.
pub fn report_error_with(command: &str, report_args: &ReportArgs, error: CommandError) -> i32 {
    if report_args.is_json() {
        let mut report = CommandReport::failure(command, 1);
        report.summary = Some(error.message.clone());
        report.error = Some(error);
        print_report(&report, report_args.format());
    } else {
        console::error(&format!("error: {}", error.message));
    }
    1
}

/// Decode a structured daemon payload into a typed payload and raw JSON value.
pub fn parse_command_payload<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    payload: Option<&BinaryPayload>,
    payload_label: &str,
    required: bool,
) -> Result<Option<(T, Value)>, i32> {
    // return early when the payload is optional and missing
    let payload = match payload {
        Some(payload) => payload,
        None => {
            if required {
                let message = format!("{payload_label} payload missing");
                return Err(report_error(command, report_args, &message));
            }
            return Ok(None);
        }
    };

    // decode the payload as json
    let value = match payload.to_json_value() {
        Ok(value) => value,
        Err(error) => {
            let message = format!("invalid {payload_label} payload: {error}");
            return Err(report_error(command, report_args, &message));
        }
    };

    // deserialize into the typed payload
    let parsed = match serde_json::from_value(value.clone()) {
        Ok(payload) => payload,
        Err(error) => {
            let message = format!("invalid {payload_label} payload: {error}");
            return Err(report_error(command, report_args, &message));
        }
    };

    Ok(Some((parsed, value)))
}

/// Decode a required daemon payload into a typed payload and raw JSON value.
pub fn parse_required_command_payload<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    payload: Option<&BinaryPayload>,
    payload_label: &str,
) -> Result<(T, Value), i32> {
    // decode the payload with the required flag
    let payload = parse_command_payload::<T>(command, report_args, payload, payload_label, true)?;

    // guard against missing payloads
    match payload {
        Some(payload) => Ok(payload),
        None => Err(report_error(
            command,
            report_args,
            &format!("{payload_label} payload missing"),
        )),
    }
}

/// Build a command report from optional payload data.
pub fn report_from_payload(
    command: &str,
    exit_code: i32,
    data: Option<Value>,
    summary: Option<String>,
    error: Option<CommandError>,
) -> CommandReport {
    // build base report from the exit code
    let mut report = if exit_code == 0 {
        CommandReport::success(command, exit_code)
    } else {
        CommandReport::failure(command, exit_code)
    };
    report.data = data;
    report.summary = summary;
    report.error = error;
    report
}

/// Build a command report from a standard message payload.
pub fn report_from_message_payload(
    command: &str,
    exit_code: i32,
    payload: Option<(CommandMessagePayload, Value)>,
) -> CommandReport {
    // derive payload-specific report fields
    let (summary, error, data) = match payload {
        Some((payload, value)) => {
            let summary = Some(payload.message.clone());
            let error = if exit_code == 0 {
                None
            } else {
                Some(CommandError::new(
                    format!("{command}_failed"),
                    "command",
                    payload.message,
                ))
            };
            (summary, error, Some(value))
        }
        None => (None, None, None),
    };

    report_from_payload(command, exit_code, data, summary, error)
}

/// Print a JSON report with a serialized payload.
pub fn print_json_payload_report<T: Serialize>(
    command: &str,
    report_args: &ReportArgs,
    exit_code: i32,
    payload: &T,
) -> Result<(), i32> {
    // skip if json output is not requested
    if !report_args.is_json() {
        return Ok(());
    }

    // serialize the payload for the report
    let data = match serde_json::to_value(payload) {
        Ok(data) => data,
        Err(error) => {
            return Err(report_error(
                command,
                report_args,
                &format!("failed to serialize payload: {error}"),
            ));
        }
    };

    // emit the report payload
    let report = report_from_payload(command, exit_code, Some(data), None, None);
    print_report(&report, report_args.format());
    Ok(())
}

/// Report a missing input error with optional usage help.
pub fn report_no_input(command: &str, report_args: &ReportArgs) -> i32 {
    if report_args.is_json() {
        return report_error_with(
            command,
            report_args,
            CommandError::new("no_input", "usage", "no input provided"),
        );
    }

    print_no_input_help(command);
    1
}

/// Summary info for stats output.
#[derive(Debug)]
pub struct StatsSummary<'a> {
    /// Action verb to display, e.g. "Checked", "Built", "Linted".
    pub verb: &'a str,
    /// Number of modules processed.
    pub modules: usize,
    /// Number of profiles used.
    pub profiles: usize,
    /// Number of targets built.
    pub targets: usize,
    /// Number of errors found.
    pub errors: usize,
    /// Number of warnings found.
    pub warnings: usize,
}

/// Print a stats summary line after diagnostics.
pub fn print_stats_summary(
    summary: &StatsSummary<'_>,
    stats: &StatsSnapshot,
    timing_options: TimingOutputOptions,
    line_writer: Option<&LineWriter>,
) {
    // compute elapsed seconds for throughput calculations
    let elapsed_secs = stats.elapsed.as_secs_f64();

    // counts: "N modules[, M profiles][, K targets]"
    let mut parts = Vec::new();
    parts.push(pluralize(summary.modules, "module"));
    if summary.profiles > 1 {
        parts.push(pluralize(summary.profiles, "profile"));
    }
    if summary.targets > 0 {
        parts.push(pluralize(summary.targets, "target"));
    }
    let counts = parts.join(", ");

    // build main message and colorize based on status
    let elapsed_str = console::format_duration(stats.elapsed);
    let (icon, main_part) = if summary.errors > 0 {
        let status = format!(" with {}", pluralize(summary.errors, "error"));
        let main = format!("{} {counts}{status} in {elapsed_str}", summary.verb);
        // bold and bright red
        (
            console::failure_icon(),
            console::style_for_stream(&main, &["1", "91"], console::Stream::Stderr),
        )
    } else if summary.warnings > 0 {
        let status = format!(" with {}", pluralize(summary.warnings, "warning"));
        let main = format!("{} {counts}{status} in {elapsed_str}", summary.verb);
        // bold and bright yellow
        (
            console::success_icon(),
            console::style_for_stream(&main, &["1", "93"], console::Stream::Stderr),
        )
    } else {
        let main = format!("{} {counts} in {elapsed_str}", summary.verb);
        // bold and green
        (
            console::success_icon(),
            console::style_for_stream(&main, &["1", "32"], console::Stream::Stderr),
        )
    };

    // line count with throughput, bold only after the dot
    let lines_suffix = if stats.modules.lines_processed > 0 && elapsed_secs > 0.001 {
        let throughput = stats.modules.lines_processed as f64 / elapsed_secs;
        console::bold(&format!(
            " · {} lines · {} lines/s",
            format_number(stats.modules.lines_processed),
            format_compact(throughput as usize)
        ))
    } else if stats.modules.lines_processed > 0 {
        console::bold(&format!(
            " · {} lines",
            format_number(stats.modules.lines_processed)
        ))
    } else {
        String::new()
    };

    write_line(line_writer, &format!("{icon} {main_part}{lines_suffix}"));

    // cache summary when activity is present
    let cache_totals = stats.cache_totals();
    let cache_activity = cache_totals.hits_memory
        + cache_totals.hits_disk
        + cache_totals.misses
        + cache_totals.writes_memory
        + cache_totals.writes_disk
        + cache_totals.errors;
    if cache_activity > 0 {
        let hits = cache_totals.hits_memory + cache_totals.hits_disk;
        let writes = cache_totals.writes_memory + cache_totals.writes_disk;
        let hit_rate = stats.cache_hit_rate() * 100.0;
        let cache_line = format!(
            "cache: {} hits ({} mem, {} disk) · {} misses · {} writes · {} errors · {:.0}% hit rate",
            format_number(hits),
            format_number(cache_totals.hits_memory),
            format_number(cache_totals.hits_disk),
            format_number(cache_totals.misses),
            format_number(writes),
            format_number(cache_totals.errors),
            hit_rate,
        );
        write_line(line_writer, &console::dim(&cache_line));
    }

    // show mir optimization metrics if any optimizations were performed
    if stats.mir.functions_optimized > 0 && stats.mir.instructions_before > 0 {
        let instr_before = stats.mir.instructions_before;
        let instr_after = stats.mir.instructions_after;
        let blocks_before = stats.mir.blocks_before;
        let blocks_after = stats.mir.blocks_after;

        // calculate percentages, negative means reduction
        let instr_delta = if instr_before > 0 {
            ((instr_after as f64 - instr_before as f64) / instr_before as f64 * 100.0) as i32
        } else {
            0
        };
        let blocks_delta = if blocks_before > 0 {
            ((blocks_after as f64 - blocks_before as f64) / blocks_before as f64 * 100.0) as i32
        } else {
            0
        };

        // format delta with sign and color
        let format_delta = |delta: i32| -> String {
            if delta < 0 {
                console::green(&format!("{delta}%"))
            } else if delta > 0 {
                console::yellow(&format!("+{delta}%"))
            } else {
                console::dim("0%")
            }
        };

        let arrow = console::SYMBOL_ARROW;
        let mir_line = format!(
            "    {} optimized {} functions: {} {arrow} {} instructions ({}), {} {arrow} {} blocks ({})",
            console::dim(console::SYMBOL_ARROW),
            stats.mir.functions_optimized,
            format_number(instr_before),
            format_number(instr_after),
            format_delta(instr_delta),
            format_number(blocks_before),
            format_number(blocks_after),
            format_delta(blocks_delta),
        );
        write_line(line_writer, &mir_line);
    }

    // show per package breakdown if multiple packages, excluding internal ones
    // aggregate by package name to avoid duplicates
    let mut package_map: HashMap<String, (usize, usize, Duration)> = HashMap::new();
    for pkg in &stats.packages {
        let name = pkg.name.as_deref().unwrap_or("");
        // skip internal packages
        if name.starts_with('<') || pkg.lines == 0 {
            continue;
        }
        let entry = package_map
            .entry(name.to_string())
            .or_insert((0, 0, Duration::ZERO));
        entry.0 += pkg.modules;
        entry.1 += pkg.lines;
        entry.2 += pkg.duration;
    }

    if !package_map.is_empty() {
        // sort: builtin packages last, then by name
        let mut packages: Vec<_> = package_map.into_iter().collect();
        packages.sort_by(|(a_name, _), (b_name, _)| {
            let a_is_builtin = a_name.contains("builtin");
            let b_is_builtin = b_name.contains("builtin");

            match (a_is_builtin, b_is_builtin) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a_name.cmp(b_name),
            }
        });

        for (name, (modules, lines, duration)) in packages {
            let duration_str = if duration.as_nanos() > 0 {
                console::cyan(&console::format_duration(duration))
            } else {
                String::new()
            };
            let modules_str = pluralize(modules, "module");
            let lines_str = format!("{} lines", format_number(lines));
            let throughput_str = if duration.as_nanos() > 0 {
                let duration_secs = duration.as_secs_f64();
                if lines > 0 && duration_secs > 0.001 {
                    let throughput = lines as f64 / duration_secs;
                    format!(" · {} lines/s", format_compact(throughput as usize))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            write_line(
                line_writer,
                &format!(
                    "    {}  {} · {} · {}{}",
                    console::cyan(&name),
                    duration_str,
                    modules_str,
                    lines_str,
                    throughput_str
                ),
            );
        }
    }

    // show per phase timing, labels dimmed, times in cyan
    // skip phases with zero duration
    let visible_phases: Vec<_> = stats
        .phases
        .iter()
        .filter(|p| p.duration.as_nanos() > 0)
        .collect();

    if !visible_phases.is_empty() {
        let phase_parts: Vec<String> = visible_phases
            .iter()
            .map(|p| {
                let name = console::dim(p.phase.name());
                let duration = console::cyan(&console::format_duration(p.duration));
                format!("{name} {duration}")
            })
            .collect();

        let sep = console::dim(" · ");
        write_line(line_writer, &format!("    {}", phase_parts.join(&sep)));
    }

    // show timing tag summary when requested
    if timing_options.enabled {
        let timing_entries = timing_entries_from_snapshot(stats, timing_options);
        write_timing_summary(line_writer, &timing_entries, timing_options);
    }
}

/// Print a stats summary line for daemon command payloads.
pub fn print_command_stats_summary(
    summary: &StatsSummary<'_>,
    stats: &CommandStats,
    timing_options: TimingOutputOptions,
    line_writer: Option<&LineWriter>,
) {
    // compute elapsed seconds for throughput calculations
    let elapsed = std::time::Duration::from_millis(stats.elapsed_ms);
    let elapsed_secs = elapsed.as_secs_f64();

    // counts: "N modules[, M profiles][, K targets]"
    let mut parts = Vec::new();
    parts.push(pluralize(summary.modules, "module"));
    if summary.profiles > 1 {
        parts.push(pluralize(summary.profiles, "profile"));
    }
    if summary.targets > 0 {
        parts.push(pluralize(summary.targets, "target"));
    }
    let counts = parts.join(", ");

    // build main message and colorize based on status
    let elapsed_str = console::format_duration(elapsed);
    let (icon, main_part) = if summary.errors > 0 {
        let status = format!(" with {}", pluralize(summary.errors, "error"));
        let main = format!("{} {counts}{status} in {elapsed_str}", summary.verb);
        (
            console::failure_icon(),
            console::style_for_stream(&main, &["1", "91"], console::Stream::Stderr),
        )
    } else if summary.warnings > 0 {
        let status = format!(" with {}", pluralize(summary.warnings, "warning"));
        let main = format!("{} {counts}{status} in {elapsed_str}", summary.verb);
        (
            console::success_icon(),
            console::style_for_stream(&main, &["1", "93"], console::Stream::Stderr),
        )
    } else {
        let main = format!("{} {counts} in {elapsed_str}", summary.verb);
        (
            console::success_icon(),
            console::style_for_stream(&main, &["1", "32"], console::Stream::Stderr),
        )
    };

    // line count with throughput, bold only after the dot
    let lines_suffix = if stats.lines_processed > 0 && elapsed_secs > 0.001 {
        let throughput = stats.lines_processed as f64 / elapsed_secs;
        console::bold(&format!(
            " · {} lines · {} lines/s",
            format_number(stats.lines_processed),
            format_compact(throughput as usize)
        ))
    } else if stats.lines_processed > 0 {
        console::bold(&format!(
            " · {} lines",
            format_number(stats.lines_processed)
        ))
    } else {
        String::new()
    };

    write_line(line_writer, &format!("{icon} {main_part}{lines_suffix}"));

    // cache summary when activity is present
    if let Some(cache) = stats.cache.as_ref() {
        let hits = cache.hits_memory + cache.hits_disk;
        let writes = cache.writes_memory + cache.writes_disk;
        let hit_rate = cache.hit_rate * 100.0;
        let cache_line = format!(
            "cache: {} hits ({} mem, {} disk) · {} misses · {} writes · {} errors · {:.0}% hit rate",
            format_number(hits),
            format_number(cache.hits_memory),
            format_number(cache.hits_disk),
            format_number(cache.misses),
            format_number(writes),
            format_number(cache.errors),
            hit_rate,
        );
        write_line(line_writer, &console::dim(&cache_line));
    }

    // show timing tag summary when requested
    if timing_options.enabled {
        let timing_entries = timing_entries_from_command_stats(stats, timing_options);
        write_timing_summary(line_writer, &timing_entries, timing_options);
    }
}

#[derive(Debug, Clone)]
struct TimingEntry {
    name: String,
    duration: Duration,
    sample_count: u64,
}

fn timing_entries_from_snapshot(
    stats: &StatsSnapshot,
    timing_options: TimingOutputOptions,
) -> Vec<TimingEntry> {
    let min_ms = timing_options.min_ms as u128;
    let mut entries: Vec<TimingEntry> = stats
        .timings
        .iter()
        .filter(|entry| entry.duration.as_millis() >= min_ms)
        .map(|entry| TimingEntry {
            name: entry.name.clone(),
            duration: entry.duration,
            sample_count: entry.sample_count as u64,
        })
        .collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.duration));
    if timing_options.top > 0 && entries.len() > timing_options.top {
        entries.truncate(timing_options.top);
    }
    entries
}

fn timing_entries_from_command_stats(
    stats: &CommandStats,
    timing_options: TimingOutputOptions,
) -> Vec<TimingEntry> {
    let Some(timings) = stats.timings.as_ref() else {
        return Vec::new();
    };
    let min_ms = timing_options.min_ms;
    let mut entries: Vec<TimingEntry> = timings
        .iter()
        .filter(|entry| entry.duration_ms >= min_ms)
        .map(|entry| TimingEntry {
            name: entry.name.clone(),
            duration: Duration::from_millis(entry.duration_ms),
            sample_count: entry.sample_count,
        })
        .collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.duration));
    if timing_options.top > 0 && entries.len() > timing_options.top {
        entries.truncate(timing_options.top);
    }
    entries
}

fn write_timing_summary(
    line_writer: Option<&LineWriter>,
    timing_entries: &[TimingEntry],
    timing_options: TimingOutputOptions,
) {
    if timing_entries.is_empty() {
        return;
    }

    let top_label = if timing_options.top == 0 {
        "all".to_string()
    } else {
        timing_options.top.to_string()
    };
    let header = format!(
        "timing tags: top {} (min {}ms)",
        top_label, timing_options.min_ms
    );
    write_line(line_writer, &console::dim(&header));

    for entry in timing_entries {
        let name = console::dim(&entry.name);
        let duration = console::cyan(&console::format_duration(entry.duration));
        let count = console::dim(&format!("{}x", entry.sample_count));
        write_line(line_writer, &format!("    {name} {duration} · {count}"));
    }
}

/// Write a line using the optional writer.
fn write_line(line_writer: Option<&LineWriter>, line: &str) {
    // use the line writer when provided
    if let Some(writer) = line_writer {
        writer(line);
    } else {
        eprintln!("{line}");
    }
}

/// Format a number with grouping separators.
fn format_number(n: usize) -> String {
    let mut digits = n.to_string();
    let mut output = String::new();
    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        if output.is_empty() {
            output = tail;
        } else {
            output = format!("{tail},{output}");
        }
    }
    if output.is_empty() {
        digits
    } else {
        format!("{digits},{output}")
    }
}

/// Format a number in a compact human friendly form.
fn format_compact(n: usize) -> String {
    let n = n as f64;
    if n >= 1_000_000_000.0 {
        format!("{:.1}b", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("{:.1}m", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else {
        format!("{n:.0}")
    }
}

/// Pluralize a word based on the provided count.
fn pluralize(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// Convert a duration to milliseconds with saturation.
fn duration_to_ms(duration: Duration) -> u64 {
    // guard against overflow on large durations
    let millis = duration.as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[cfg(feature = "schema")]
fn schema_any(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    true.into()
}
