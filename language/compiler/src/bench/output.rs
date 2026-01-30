use std::time::Duration;

use crate::bench::runner::{BenchMode, BenchOptions, BenchOutputFormat, LibTiming};

const COLUMN_GAP: &str = "  ";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

/// Emit output for the bench results.
pub(super) fn output_results(timings: &[LibTiming], options: &BenchOptions) {
    match options.output {
        BenchOutputFormat::Table => {
            output_table(timings, options);
        }
        BenchOutputFormat::Json => {
            output_json(timings, options);
        }
        BenchOutputFormat::Csv => {
            output_csv(timings);
        }
    }

    // write csv output if requested
    if let Some(path) = &options.csv_path
        && let Err(error) = write_timings_csv(timings, path)
    {
        eprintln!("timings: failed to write csv to {path}: {error}");
    }
}

/// Format a duration for readable timing output.
pub(super) fn format_duration(duration: Duration) -> String {
    let ms = duration.as_secs_f64() * 1000.0;
    if ms >= 1000.0 {
        return format!("{:.3}s", ms / 1000.0);
    }

    format!("{ms:.3}ms")
}

/// Format a percentage for output.
fn format_percent(value: f64) -> String {
    if value >= 100.0 {
        return format!("{value:.0}%");
    }

    format!("{value:.1}%")
}

/// Escape a string for json output.
fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Escape a string for csv output.
fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Emit results as json.
fn output_json(timings: &[LibTiming], options: &BenchOptions) {
    let mut output = String::new();
    output.push_str("{\"mode\":\"");
    output.push_str(options.mode.label());
    output.push_str("\",\"run\":\"");
    output.push_str(&format!("{:?}", options.run).to_ascii_lowercase());
    output.push_str("\",\"results\":[");

    for (index, timing) in timings.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"lib\":\"");
        output.push_str(&json_escape(&timing.name));
        output.push_str("\",\"import_ms\":");
        output.push_str(&format!("{:.3}", timing.import.as_secs_f64() * 1000.0));
        output.push_str(",\"resolve_ms\":");
        output.push_str(&format!("{:.3}", timing.resolve.as_secs_f64() * 1000.0));
        output.push_str(",\"analyze_ms\":");
        output.push_str(&format!("{:.3}", timing.analyze.as_secs_f64() * 1000.0));
        output.push_str(",\"total_ms\":");
        output.push_str(&format!("{:.3}", timing.total.as_secs_f64() * 1000.0));
        output.push('}');
    }

    output.push_str("]}");
    println!("{output}");
}

/// Emit results as csv.
fn output_csv(timings: &[LibTiming]) {
    println!("lib,import_ms,resolve_ms,analyze_ms,total_ms");
    for timing in timings {
        println!(
            "{},{:.3},{:.3},{:.3},{:.3}",
            csv_escape(&timing.name),
            timing.import.as_secs_f64() * 1000.0,
            timing.resolve.as_secs_f64() * 1000.0,
            timing.analyze.as_secs_f64() * 1000.0,
            timing.total.as_secs_f64() * 1000.0
        );
    }
}

/// Table column widths for table output.
struct TableWidths {
    /// Width for the lib column.
    lib: usize,
    /// Width for the import column.
    import: usize,
    /// Width for the resolve column.
    resolve: usize,
    /// Width for the analyze column.
    analyze: usize,
    /// Width for the total column.
    total: usize,
    /// Width for the share column.
    share: usize,
}

impl TableWidths {
    /// Build base widths from header labels.
    fn new() -> Self {
        Self {
            lib: "Library".len().max(12),
            import: "Import".len().max(8),
            resolve: "Resolve".len().max(8),
            analyze: "Analyze".len().max(8),
            total: "Total".len().max(8),
            share: "Share".len().max(7),
        }
    }

    /// Expand widths to fit row data.
    fn update_with_row(&mut self, row: &TableRow) {
        self.lib = self.lib.max(row.name.len());
        self.import = self.import.max(row.import.len());
        self.resolve = self.resolve.max(row.resolve.len());
        self.analyze = self.analyze.max(row.analyze.len());
        self.total = self.total.max(row.total.len());
        self.share = self.share.max(row.share.len());
    }

    /// Return the full table width in characters.
    fn total_width(&self) -> usize {
        self.lib
            + self.import
            + self.resolve
            + self.analyze
            + self.total
            + self.share
            + COLUMN_GAP.len() * 5
    }
}

/// Formatted row for table output.
struct TableRow {
    /// The library name.
    name: String,
    /// Import duration string.
    import: String,
    /// Resolve duration string.
    resolve: String,
    /// Analyze duration string.
    analyze: String,
    /// Total duration string.
    total: String,
    /// Share string.
    share: String,
    /// Total duration for color scaling.
    total_duration: Duration,
}

/// Style helpers for table output.
struct TableStyle {
    /// Bold text prefix.
    bold: &'static str,
    /// Dim text prefix.
    dim: &'static str,
    /// Reset code.
    reset: &'static str,
    /// Cyan text prefix.
    cyan: &'static str,
    /// Green text prefix.
    green: &'static str,
    /// Yellow text prefix.
    yellow: &'static str,
    /// Red text prefix.
    red: &'static str,
}

impl TableStyle {
    /// Build a table style with or without ANSI codes.
    fn new(color: bool) -> Self {
        if color {
            return Self {
                bold: BOLD,
                dim: DIM,
                reset: RESET,
                cyan: CYAN,
                green: GREEN,
                yellow: YELLOW,
                red: RED,
            };
        }

        Self {
            bold: "",
            dim: "",
            reset: "",
            cyan: "",
            green: "",
            yellow: "",
            red: "",
        }
    }

    /// Pick a color for totals based on range.
    fn color_for_total(&self, value: Duration, min: Duration, max: Duration) -> &'static str {
        if self.green.is_empty() {
            return "";
        }

        if max <= min {
            return self.green;
        }

        let value_ms = value.as_secs_f64();
        let min_ms = min.as_secs_f64();
        let max_ms = max.as_secs_f64();
        let ratio = (value_ms - min_ms) / (max_ms - min_ms);

        if ratio >= 0.8 {
            self.red
        } else if ratio >= 0.5 {
            self.yellow
        } else {
            self.green
        }
    }
}

/// Emit results as a formatted table.
fn output_table(timings: &[LibTiming], options: &BenchOptions) {
    let mut sorted = timings.to_vec();
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.total));

    let total_import = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.import);
    let total_resolve = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.resolve);
    let total_analyze = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.analyze);
    let total_all = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.total);

    let min_total = timings
        .iter()
        .map(|entry| entry.total)
        .min()
        .unwrap_or(Duration::ZERO);
    let max_total = timings
        .iter()
        .map(|entry| entry.total)
        .max()
        .unwrap_or(Duration::ZERO);

    let show_count = options.report_top_n.min(sorted.len());
    let truncated = show_count < sorted.len();

    // preformat rows for consistent widths
    let rows = if truncated {
        sorted.iter().take(show_count).collect::<Vec<_>>()
    } else {
        sorted.iter().collect::<Vec<_>>()
    };

    let mut table_rows = Vec::with_capacity(rows.len());
    for timing in rows {
        let share = if total_all.is_zero() {
            0.0
        } else {
            timing.total.as_secs_f64() / total_all.as_secs_f64() * 100.0
        };
        table_rows.push(TableRow {
            name: timing.name.clone(),
            import: format_duration(timing.import),
            resolve: format_duration(timing.resolve),
            analyze: format_duration(timing.analyze),
            total: format_duration(timing.total),
            share: format_percent(share),
            total_duration: timing.total,
        });
    }

    let mut widths = TableWidths::new();
    for row in &table_rows {
        widths.update_with_row(row);
    }

    let style = TableStyle::new(options.color);
    let table_width = widths.total_width();

    println!();
    println!(
        "{bold}{:<lib_width$}{reset}{gap}{:>import_width$}{gap}{:>resolve_width$}{gap}{:>analyze_width$}{gap}{:>total_width$}{gap}{:>share_width$}{reset}",
        "Library",
        "Import",
        "Resolve",
        "Analyze",
        "Total",
        "Share",
        lib_width = widths.lib,
        import_width = widths.import,
        resolve_width = widths.resolve,
        analyze_width = widths.analyze,
        total_width = widths.total,
        share_width = widths.share,
        gap = COLUMN_GAP,
        bold = style.bold,
        reset = style.reset,
    );

    println!(
        "{dim}{}{reset}",
        "─".repeat(table_width),
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "{dim}mode {} · {:?} · validate={} · libs={} · timeout={:?}{reset}",
        options.mode.label(),
        options.run,
        options.validate_builtin_libs,
        timings.len(),
        options.effective_timeout(),
        dim = style.dim,
        reset = style.reset
    );
    if matches!(options.mode, BenchMode::Sequential)
        && options.timeout != options.effective_timeout()
    {
        println!(
            "{dim}timeout scaled from {:?}{reset}",
            options.timeout,
            dim = style.dim,
            reset = style.reset
        );
    }
    if truncated {
        println!(
            "{dim}showing top {show_count} by total{reset}",
            dim = style.dim,
            reset = style.reset
        );
    }
    println!(
        "{dim}{}{reset}",
        "─".repeat(table_width),
        dim = style.dim,
        reset = style.reset
    );

    for row in &table_rows {
        let total_color = style.color_for_total(row.total_duration, min_total, max_total);
        println!(
            "{cyan}{:<lib_width$}{reset}{gap}{:>import_width$}{gap}{:>resolve_width$}{gap}{:>analyze_width$}{gap}{total_color}{:>total_width$}{reset}{gap}{dim}{:>share_width$}{reset}",
            row.name,
            row.import,
            row.resolve,
            row.analyze,
            row.total,
            row.share,
            lib_width = widths.lib,
            import_width = widths.import,
            resolve_width = widths.resolve,
            analyze_width = widths.analyze,
            total_width = widths.total,
            share_width = widths.share,
            gap = COLUMN_GAP,
            cyan = style.cyan,
            total_color = total_color,
            dim = style.dim,
            reset = style.reset,
        );
    }

    println!(
        "{dim}{}{reset}",
        "─".repeat(table_width),
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "{bold}{:<lib_width$}{reset}{gap}{:>import_width$}{gap}{:>resolve_width$}{gap}{:>analyze_width$}{gap}{:>total_width$}{reset}{gap}{:>share_width$}{reset}",
        "Total",
        format_duration(total_import),
        format_duration(total_resolve),
        format_duration(total_analyze),
        format_duration(total_all),
        format_percent(100.0),
        lib_width = widths.lib,
        import_width = widths.import,
        resolve_width = widths.resolve,
        analyze_width = widths.analyze,
        total_width = widths.total,
        share_width = widths.share,
        gap = COLUMN_GAP,
        bold = style.bold,
        reset = style.reset,
    );
    println!(
        "{dim}{}{reset}",
        "─".repeat(table_width),
        dim = style.dim,
        reset = style.reset
    );
}

/// Write timing results to a CSV file.
fn write_timings_csv(timings: &[LibTiming], path: &str) -> std::io::Result<()> {
    // build CSV output in memory
    let mut output = String::new();
    output.push_str("lib,import_ms,resolve_ms,analyze_ms,total_ms\n");
    for entry in timings {
        output.push_str(&format!(
            "{},{:.3},{:.3},{:.3},{:.3}\n",
            csv_escape(&entry.name),
            entry.import.as_secs_f64() * 1000.0,
            entry.resolve.as_secs_f64() * 1000.0,
            entry.analyze.as_secs_f64() * 1000.0,
            entry.total.as_secs_f64() * 1000.0
        ));
    }

    // write CSV output to disk
    std::fs::write(path, output)
}
