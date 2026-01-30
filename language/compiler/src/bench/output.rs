use std::time::Duration;

use crate::bench::runner::{BenchMode, BenchOptions, BenchOutputFormat, LibTiming};

const COLUMN_GAP: &str = "   ";
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

/// Format a count with digit grouping.
fn format_count(value: usize) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Format a mean line count with a single decimal.
fn format_mean_lines(value: f64) -> String {
    let formatted = format!("{value:.1}");
    let Some((int_part, frac_part)) = formatted.split_once('.') else {
        return formatted;
    };
    let int_value = int_part.parse::<usize>().unwrap_or(0);
    format!("{}.{}", format_count(int_value), frac_part)
}

/// Format a lines per second rate with a single decimal.
fn format_lines_per_sec(value: f64) -> String {
    let formatted = format!("{value:.1}");
    let Some((int_part, frac_part)) = formatted.split_once('.') else {
        return format!("{formatted}/s");
    };
    let int_value = int_part.parse::<usize>().unwrap_or(0);
    format!("{}.{}{}", format_count(int_value), frac_part, "/s")
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
        output.push_str("\",\"modules\":");
        output.push_str(&timing.module_count.to_string());
        output.push_str(",\"lines_total\":");
        output.push_str(&timing.total_lines.to_string());
        output.push_str(",\"lines_min\":");
        output.push_str(&timing.min_lines.to_string());
        output.push_str(",\"lines_max\":");
        output.push_str(&timing.max_lines.to_string());
        output.push_str(",\"lines_mean\":");
        output.push_str(&format!("{:.1}", timing.mean_lines));
        output.push_str(",\"lines_per_sec\":");
        output.push_str(&format!(
            "{:.1}",
            timing.total_lines as f64 / timing.total.as_secs_f64().max(0.000_001)
        ));
        output.push_str(",\"import_ms\":");
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
    println!(
        "lib,modules,lines_total,lines_min,lines_max,lines_mean,lines_per_sec,import_ms,resolve_ms,analyze_ms,total_ms"
    );
    for timing in timings {
        println!(
            "{},{},{},{},{},{:.1},{:.1},{:.3},{:.3},{:.3},{:.3}",
            csv_escape(&timing.name),
            timing.module_count,
            timing.total_lines,
            timing.min_lines,
            timing.max_lines,
            timing.mean_lines,
            timing.total_lines as f64 / timing.total.as_secs_f64().max(0.000_001),
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
    /// Width for the modules column.
    modules: usize,
    /// Width for the lines column.
    lines: usize,
    /// Width for the min lines column.
    min_lines: usize,
    /// Width for the max lines column.
    max_lines: usize,
    /// Width for the mean lines column.
    mean_lines: usize,
    /// Width for the lines per second column.
    lines_per_sec: usize,
}

impl TableWidths {
    /// Build base widths from header labels.
    fn new() -> Self {
        Self {
            lib: "Library".len().max(12),
            import: "Import".len().max(9),
            resolve: "Resolve".len().max(9),
            analyze: "Analyze".len().max(9),
            total: "Total".len().max(9),
            modules: "Modules".len().max(8),
            lines: "Lines".len().max(9),
            min_lines: "Min".len().max(5),
            max_lines: "Max".len().max(5),
            mean_lines: "Mean".len().max(6),
            lines_per_sec: "Lines/s".len().max(10),
        }
    }

    /// Expand widths to fit row data.
    fn update_with_row(&mut self, row: &TableRow) {
        self.lib = self.lib.max(row.name.len());
        self.import = self.import.max(row.import.len());
        self.resolve = self.resolve.max(row.resolve.len());
        self.analyze = self.analyze.max(row.analyze.len());
        self.total = self.total.max(row.total.len());
        self.modules = self.modules.max(row.modules.len());
        self.lines = self.lines.max(row.lines.len());
        self.min_lines = self.min_lines.max(row.min_lines.len());
        self.max_lines = self.max_lines.max(row.max_lines.len());
        self.mean_lines = self.mean_lines.max(row.mean_lines.len());
        self.lines_per_sec = self.lines_per_sec.max(row.lines_per_sec.len());
    }

    /// Return the full table width in characters.
    fn total_width(&self) -> usize {
        self.lib
            + self.import
            + self.resolve
            + self.analyze
            + self.total
            + self.modules
            + self.lines
            + self.min_lines
            + self.max_lines
            + self.mean_lines
            + self.lines_per_sec
            + COLUMN_GAP.len() * 10
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
    /// Modules count string.
    modules: String,
    /// Total lines string.
    lines: String,
    /// Min lines string.
    min_lines: String,
    /// Max lines string.
    max_lines: String,
    /// Mean lines string.
    mean_lines: String,
    /// Lines per second string.
    lines_per_sec: String,
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
    let total_modules = timings.iter().map(|entry| entry.module_count).sum::<usize>();
    let total_lines = timings.iter().map(|entry| entry.total_lines).sum::<usize>();
    let min_lines = timings.iter().map(|entry| entry.min_lines).min().unwrap_or(0);
    let max_lines = timings.iter().map(|entry| entry.max_lines).max().unwrap_or(0);
    let mean_lines = if total_modules == 0 {
        0.0
    } else {
        total_lines as f64 / total_modules as f64
    };

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
        table_rows.push(TableRow {
            name: timing.name.clone(),
            import: format_duration(timing.import),
            resolve: format_duration(timing.resolve),
            analyze: format_duration(timing.analyze),
            total: format_duration(timing.total),
            modules: format_count(timing.module_count),
            lines: format_count(timing.total_lines),
            min_lines: format_count(timing.min_lines),
            max_lines: format_count(timing.max_lines),
            mean_lines: format_mean_lines(timing.mean_lines),
            lines_per_sec: format_lines_per_sec(
                timing.total_lines as f64 / timing.total.as_secs_f64().max(0.000_001),
            ),
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
        "{bold}{lib:<lib_width$}{reset}{gap}{import:>import_width$}{gap}{resolve:>resolve_width$}{gap}{analyze:>analyze_width$}{gap}{total:>total_width$}{gap}{modules:>modules_width$}{gap}{lines:>lines_width$}{gap}{min:>min_lines_width$}{gap}{max:>max_lines_width$}{gap}{mean:>mean_lines_width$}{gap}{lines_per_sec:>lines_per_sec_width$}{reset}",
        lib = "Library",
        import = "Import",
        resolve = "Resolve",
        analyze = "Analyze",
        total = "Total",
        modules = "Modules",
        lines = "Lines",
        min = "Min",
        max = "Max",
        mean = "Mean",
        lines_per_sec = "Lines/s",
        lib_width = widths.lib,
        import_width = widths.import,
        resolve_width = widths.resolve,
        analyze_width = widths.analyze,
        total_width = widths.total,
        modules_width = widths.modules,
        lines_width = widths.lines,
        min_lines_width = widths.min_lines,
        max_lines_width = widths.max_lines,
        mean_lines_width = widths.mean_lines,
        lines_per_sec_width = widths.lines_per_sec,
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
            "{cyan}{lib:<lib_width$}{reset}{gap}{import:>import_width$}{gap}{resolve:>resolve_width$}{gap}{analyze:>analyze_width$}{gap}{total_color}{total:>total_width$}{reset}{gap}{modules:>modules_width$}{gap}{lines:>lines_width$}{gap}{min:>min_lines_width$}{gap}{max:>max_lines_width$}{gap}{mean:>mean_lines_width$}{gap}{lines_per_sec:>lines_per_sec_width$}{reset}",
            lib = row.name,
            import = row.import,
            resolve = row.resolve,
            analyze = row.analyze,
            total = row.total,
            modules = row.modules,
            lines = row.lines,
            min = row.min_lines,
            max = row.max_lines,
            mean = row.mean_lines,
            lines_per_sec = row.lines_per_sec,
            lib_width = widths.lib,
            import_width = widths.import,
            resolve_width = widths.resolve,
            analyze_width = widths.analyze,
            total_width = widths.total,
            modules_width = widths.modules,
            lines_width = widths.lines,
            min_lines_width = widths.min_lines,
            max_lines_width = widths.max_lines,
            mean_lines_width = widths.mean_lines,
            lines_per_sec_width = widths.lines_per_sec,
            gap = COLUMN_GAP,
            cyan = style.cyan,
            total_color = total_color,
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
        "{bold}{lib:<lib_width$}{reset}{gap}{import:>import_width$}{gap}{resolve:>resolve_width$}{gap}{analyze:>analyze_width$}{gap}{total:>total_width$}{reset}{gap}{modules:>modules_width$}{gap}{lines:>lines_width$}{gap}{min:>min_lines_width$}{gap}{max:>max_lines_width$}{gap}{mean:>mean_lines_width$}{gap}{lines_per_sec:>lines_per_sec_width$}{reset}",
        lib = "Total",
        import = format_duration(total_import),
        resolve = format_duration(total_resolve),
        analyze = format_duration(total_analyze),
        total = format_duration(total_all),
        modules = format_count(total_modules),
        lines = format_count(total_lines),
        min = format_count(min_lines),
        max = format_count(max_lines),
        mean = format_mean_lines(mean_lines),
        lines_per_sec = format_lines_per_sec(total_lines as f64 / total_all.as_secs_f64().max(0.000_001)),
        lib_width = widths.lib,
        import_width = widths.import,
        resolve_width = widths.resolve,
        analyze_width = widths.analyze,
        total_width = widths.total,
        modules_width = widths.modules,
        lines_width = widths.lines,
        min_lines_width = widths.min_lines,
        max_lines_width = widths.max_lines,
        mean_lines_width = widths.mean_lines,
        lines_per_sec_width = widths.lines_per_sec,
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
    output.push_str("lib,modules,lines_total,lines_min,lines_max,lines_mean,import_ms,resolve_ms,analyze_ms,total_ms\n");
    for entry in timings {
        output.push_str(&format!(
            "{},{},{},{},{},{:.1},{:.3},{:.3},{:.3},{:.3}\n",
            csv_escape(&entry.name),
            entry.module_count,
            entry.total_lines,
            entry.min_lines,
            entry.max_lines,
            entry.mean_lines,
            entry.import.as_secs_f64() * 1000.0,
            entry.resolve.as_secs_f64() * 1000.0,
            entry.analyze.as_secs_f64() * 1000.0,
            entry.total.as_secs_f64() * 1000.0
        ));
    }

    // write CSV output to disk
    std::fs::write(path, output)
}
