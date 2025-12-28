//! Output formatting for diagnostics.

use std::collections::BTreeMap;
use std::sync::Arc;

use destack_parser::source_colorizer;
use destack_source::{
    Diagnostic, DiagnosticCollection, DiagnosticSeverity, FileRegistry, PrintOptions,
    print_diagnostics as print_diagnostics_impl,
};
use serde::Serialize;

use crate::console;

/// Output format for diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiagnosticFormat {
    /// Human-readable text output.
    #[default]
    Text,
    /// JSON output for tooling integration.
    Json,
    /// GitHub Actions annotations format.
    Github,
}

pub type LineWriter = Arc<dyn Fn(&str) + Send + Sync>;

/// Options for output formatting.
#[derive(Debug, Clone, Default)]
pub struct FormatOptions {
    /// The output format.
    pub format: DiagnosticFormat,
    /// Whether to suppress warnings (quiet mode).
    pub quiet: bool,
    /// Maximum warning count before failure (None = no limit).
    pub max_warnings: Option<usize>,
    /// Whether to show statistics.
    pub statistics: bool,
    /// Suppress diagnostic output entirely.
    pub suppress_diagnostics: bool,
}

/// Diagnostic output for JSON serialization.
#[derive(Serialize)]
struct DiagnosticJson {
    code: String,
    severity: String,
    message: String,
    file: String,
    line: u32,
    column: u32,
    end_line: u32,
    end_column: u32,
}

/// Statistics output for JSON serialization.
#[derive(Serialize)]
struct DiagnosticOutputJson {
    diagnostics: Vec<DiagnosticJson>,
    summary: DiagnosticSummaryJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    statistics: Option<Vec<DiagnosticStatistic>>,
}

/// Summary of diagnostics.
#[derive(Serialize)]
struct DiagnosticSummaryJson {
    errors: usize,
    warnings: usize,
    notes: usize,
}

/// Print diagnostics with the given output options.
pub fn format_diagnostics(
    files: &FileRegistry,
    diagnostics: &DiagnosticCollection,
    options: &FormatOptions,
    module_count: usize,
) -> FormatResult {
    format_diagnostics_with_writer(files, diagnostics, options, module_count, None)
}

pub fn format_diagnostics_with_writer(
    files: &FileRegistry,
    diagnostics: &DiagnosticCollection,
    options: &FormatOptions,
    module_count: usize,
    line_writer: Option<&LineWriter>,
) -> FormatResult {
    // filter diagnostics based on quiet mode
    let filtered: Vec<Diagnostic> = if options.quiet {
        diagnostics
            .iter()
            .into_iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .collect()
    } else {
        diagnostics.iter()
    };

    // count by severity
    let counts = count_by_severity(&filtered);
    let error_count = *counts.get(&DiagnosticSeverity::Error).unwrap_or(&0);
    let warning_count = *counts.get(&DiagnosticSeverity::Warning).unwrap_or(&0);
    let note_count = *counts.get(&DiagnosticSeverity::Note).unwrap_or(&0);

    if !options.suppress_diagnostics {
        match options.format {
            DiagnosticFormat::Text => {
                print_text(
                    files,
                    &filtered,
                    module_count,
                    options.statistics,
                    line_writer,
                );
            }
            DiagnosticFormat::Json => {
                print_json(
                    files,
                    &filtered,
                    error_count,
                    warning_count,
                    note_count,
                    options,
                );
            }
            DiagnosticFormat::Github => {
                print_github(files, &filtered);
            }
        }
    }

    // compute exit code
    let max_warnings_exceeded = options.max_warnings.is_some_and(|max| warning_count > max);

    FormatResult {
        error_count,
        warning_count,
        max_warnings_exceeded,
    }
}

/// Result of printing diagnostics.
#[derive(Debug)]
pub struct FormatResult {
    /// Number of errors.
    pub error_count: usize,
    /// Number of warnings.
    pub warning_count: usize,
    /// Whether --max-warnings threshold was exceeded.
    pub max_warnings_exceeded: bool,
}

impl FormatResult {
    /// Get the exit code based on the output result.
    pub fn exit_code(&self) -> i32 {
        if self.error_count > 0 || self.max_warnings_exceeded {
            1
        } else if self.warning_count > 0 {
            2
        } else {
            0
        }
    }
}

/// Print diagnostics in human-readable text format.
fn print_text(
    files: &FileRegistry,
    diagnostics: &[Diagnostic],
    module_count: usize,
    show_statistics: bool,
    line_writer: Option<&LineWriter>,
) {
    let print_options = PrintOptions::new()
        .with_line_width(100)
        .with_module_count(module_count)
        .with_colorizer(source_colorizer())
        .with_skip_summary(true);
    let print_options = if let Some(writer) = line_writer.cloned() {
        print_options.with_line_writer(writer)
    } else {
        print_options
    };

    // create a temporary collection for the print function
    let mut collection = DiagnosticCollection::new();
    for d in diagnostics {
        collection.insert(d.clone());
    }

    print_diagnostics_impl(files, &collection, print_options);

    if show_statistics && !diagnostics.is_empty() {
        print_statistics(diagnostics, line_writer);
    }
}

/// Print diagnostics in JSON format.
fn print_json(
    files: &FileRegistry,
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    note_count: usize,
    options: &FormatOptions,
) {
    let json_diagnostics: Vec<DiagnosticJson> = diagnostics
        .iter()
        .map(|d| {
            let file = files.get(d.file_id);

            // get_position returns 0-based (line, column), convert to 1-based
            let (line, column) = file
                .get_position(d.primary_span.span.start)
                .map(|(l, c)| (l + 1, c + 1))
                .unwrap_or((1, 1));
            let (end_line, end_column) = file
                .get_position(d.primary_span.span.end)
                .map(|(l, c)| (l + 1, c + 1))
                .unwrap_or((1, 1));

            DiagnosticJson {
                code: d.code.clone(),
                severity: d.severity.family_name().to_string(),
                message: d.message.clone(),
                file: file
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| file.name.clone()),
                line,
                column,
                end_line,
                end_column,
            }
        })
        .collect();

    let statistics = if options.statistics {
        Some(compute_statistics(diagnostics))
    } else {
        None
    };

    let output = DiagnosticOutputJson {
        diagnostics: json_diagnostics,
        summary: DiagnosticSummaryJson {
            errors: error_count,
            warnings: warning_count,
            notes: note_count,
        },
        statistics,
    };

    if let Ok(json) = serde_json::to_string_pretty(&output) {
        println!("{json}");
    }
}

/// Print diagnostics in GitHub Actions annotation format.
fn print_github(files: &FileRegistry, diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        let file = files.get(d.file_id);
        // get_position returns 0-based (line, column), convert to 1-based
        let (line, column) = file
            .get_position(d.primary_span.span.start)
            .map(|(l, c)| (l + 1, c + 1))
            .unwrap_or((1, 1));
        let (end_line, end_column) = file
            .get_position(d.primary_span.span.end)
            .map(|(l, c)| (l + 1, c + 1))
            .unwrap_or((1, 1));

        let file_path = file
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| file.name.clone());

        // GitHub workflow command format
        // https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions
        let level = match d.severity {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Note => "notice",
        };

        println!(
            "::{level} file={file_path},line={line},col={column},endLine={end_line},endColumn={end_column},title={code}::{message}",
            code = d.code,
            message = d.message
        );
    }
}

/// Diagnostic statistic counts.
#[derive(Serialize)]
struct DiagnosticStatistic {
    /// The code of the diagnostic.
    code: String,
    /// The count of the diagnostic.
    count: usize,
    /// The severity of the diagnostic.
    severity: String,
}

/// Print statistics grouped by rule.
fn print_statistics(diagnostics: &[Diagnostic], line_writer: Option<&LineWriter>) {
    let stats = compute_statistics(diagnostics);
    if stats.is_empty() {
        return;
    }

    write_line(line_writer, "");
    write_line(line_writer, &console::bold("Statistics by rule:"));

    for stat in stats {
        let severity_color = match stat.severity.as_str() {
            "error" => "31",   // red
            "warning" => "33", // yellow
            _ => "34",         // blue
        };
        let colored_code = console::color(&stat.code, severity_color);
        write_line(
            line_writer,
            &format!("  {}: {} occurrence(s)", colored_code, stat.count),
        );
    }
}

/// Compute statistics grouped by rule code.
fn compute_statistics(diagnostics: &[Diagnostic]) -> Vec<DiagnosticStatistic> {
    let mut by_code: BTreeMap<String, (usize, DiagnosticSeverity)> = BTreeMap::new();
    for diagnostic in diagnostics {
        let entry = by_code
            .entry(diagnostic.code.clone())
            .or_insert((0, diagnostic.severity));
        entry.0 += 1;
    }

    let mut stats: Vec<DiagnosticStatistic> = by_code
        .into_iter()
        .map(|(code, (count, severity))| DiagnosticStatistic {
            code,
            count,
            severity: severity.family_name().to_string(),
        })
        .collect();

    // sort by count descending
    stats.sort_by(|a, b| b.count.cmp(&a.count));

    stats
}

/// Count diagnostics by severity.
fn count_by_severity(diagnostics: &[Diagnostic]) -> BTreeMap<DiagnosticSeverity, usize> {
    let mut counts: BTreeMap<DiagnosticSeverity, usize> = BTreeMap::new();
    for d in diagnostics {
        *counts.entry(d.severity).or_insert(0) += 1;
    }
    counts
}

fn write_line(line_writer: Option<&LineWriter>, line: &str) {
    if let Some(writer) = line_writer {
        writer(line);
    } else {
        eprintln!("{line}");
    }
}
