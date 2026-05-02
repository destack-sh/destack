use std::collections::BTreeMap;
use std::sync::Arc;

use destack_parser::source_colorizer;
use destack_source::{
    Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticRenderError, DiagnosticSeverity,
    File, FileId, PrintOptions, print_diagnostics as print_diagnostics_impl,
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

/// Line writer hook for formatted output.
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

/// Diagnostic entry serialized in JSON output.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiagnosticJson {
    /// The diagnostic code.
    code: String,
    /// The severity label.
    severity: String,
    /// The diagnostic message.
    message: String,
    /// The file path or label.
    file: String,
    /// The 1-based starting line.
    line: u32,
    /// The 1-based starting column.
    column: u32,
    /// The 1-based ending line.
    end_line: u32,
    /// The 1-based ending column.
    end_column: u32,
}

/// Diagnostic output payload for JSON output.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiagnosticOutputJson {
    /// The diagnostics list.
    diagnostics: Vec<DiagnosticJson>,
    /// Summary counts by severity.
    summary: DiagnosticSummaryJson,
    /// Optional statistics grouped by rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    statistics: Option<Vec<DiagnosticStatistic>>,
}

/// Summary counts for diagnostics.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiagnosticSummaryJson {
    /// Total error count.
    errors: usize,
    /// Total warning count.
    warnings: usize,
    /// Total note count.
    notes: usize,
}

/// Print diagnostics with the given output options.
pub fn format_diagnostics<F>(
    file_for_id: &F,
    diagnostics: &DiagnosticCollection,
    options: &FormatOptions,
    module_count: usize,
) -> FormatResult
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    // delegate to the writer aware formatter
    format_diagnostics_with_writer(file_for_id, diagnostics, options, module_count, None)
}

/// Format diagnostics with an optional line writer.
pub fn format_diagnostics_with_writer<F>(
    file_for_id: &F,
    diagnostics: &DiagnosticCollection,
    options: &FormatOptions,
    module_count: usize,
    line_writer: Option<&LineWriter>,
) -> FormatResult
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    // filter diagnostics based on quiet mode
    let filtered = filter_diagnostics(diagnostics, options.quiet);

    // count by severity
    let (error_count, warning_count, note_count) = count_severity(&filtered);

    // emit output when diagnostics are enabled
    if !options.suppress_diagnostics {
        match options.format {
            DiagnosticFormat::Text => {
                if let Err(error) = print_text(
                    file_for_id,
                    &filtered,
                    module_count,
                    options.statistics,
                    line_writer,
                ) {
                    eprintln!("failed to render diagnostics: {error}");
                }
            }
            DiagnosticFormat::Json => {
                let output = build_diagnostic_output(
                    file_for_id,
                    &filtered,
                    error_count,
                    warning_count,
                    note_count,
                    options,
                );
                print_json(&output);
            }
            DiagnosticFormat::Github => {
                print_github(file_for_id, &filtered);
            }
        }
    }

    // compute exit code
    let max_warnings_exceeded = options.max_warnings.is_some_and(|max| warning_count > max);

    // return the format result
    FormatResult {
        error_count,
        warning_count,
        max_warnings_exceeded,
    }
}

/// Collect diagnostics into JSON output without printing.
pub fn collect_diagnostics_json<F>(
    file_for_id: &F,
    diagnostics: &DiagnosticCollection,
    options: &FormatOptions,
) -> (DiagnosticOutputJson, FormatResult)
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    // filter diagnostics based on quiet mode
    let filtered = filter_diagnostics(diagnostics, options.quiet);

    // count by severity
    let (error_count, warning_count, note_count) = count_severity(&filtered);

    // build the json payload
    let output = build_diagnostic_output(
        file_for_id,
        &filtered,
        error_count,
        warning_count,
        note_count,
        options,
    );

    // compute exit code
    let max_warnings_exceeded = options.max_warnings.is_some_and(|max| warning_count > max);
    let result = FormatResult {
        error_count,
        warning_count,
        max_warnings_exceeded,
    };

    // return payload and result
    (output, result)
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
        // prioritize errors and max warning violations
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
fn print_text<F>(
    file_for_id: &F,
    diagnostics: &[Diagnostic],
    module_count: usize,
    show_statistics: bool,
    line_writer: Option<&LineWriter>,
) -> Result<(), DiagnosticRenderError>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    // build print options
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

    // build a temporary collection for printing
    let mut collection = DiagnosticCollection::new();
    for d in diagnostics {
        collection.insert(d.clone());
    }

    // render the diagnostics
    print_diagnostics_impl(file_for_id, &collection, print_options)?;

    // emit statistics when requested
    if show_statistics && !diagnostics.is_empty() {
        print_statistics(diagnostics, line_writer);
    }

    Ok(())
}

/// Print diagnostics in JSON format.
fn print_json(output: &DiagnosticOutputJson) {
    // pretty print the json payload
    if let Ok(json) = serde_json::to_string_pretty(&output) {
        println!("{json}");
    }
}

/// Print diagnostics in GitHub Actions annotation format.
fn print_github<F>(file_for_id: &F, diagnostics: &[Diagnostic])
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    // emit github annotations per diagnostic
    for d in diagnostics {
        let primary = d.primary_label();
        let primary_span = primary.span;
        let file = resolve_label_file(file_for_id, primary);

        // get_position returns 0 based line and column
        let (line, column) = file
            .get_position(primary_span.start)
            .map(|(l, c)| (l + 1, c + 1))
            .unwrap_or((1, 1));
        let (end_line, end_column) = file
            .get_position(primary_span.end)
            .map(|(l, c)| (l + 1, c + 1))
            .unwrap_or((1, 1));

        // resolve the file label
        let file_path = file
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| file.name.clone());

        // map severity to github annotation level
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
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiagnosticStatistic {
    /// The code of the diagnostic.
    code: String,
    /// The count of the diagnostic.
    count: usize,
    /// The severity of the diagnostic.
    severity: String,
}

/// Print statistics grouped by rule.
fn print_statistics(diagnostics: &[Diagnostic], line_writer: Option<&LineWriter>) {
    // compute summary counts
    let stats = compute_statistics(diagnostics);
    if stats.is_empty() {
        return;
    }

    // emit the header
    write_line(line_writer, "");
    write_line(line_writer, &console::bold("Statistics by rule:"));

    // emit one line per rule
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

/// Filter diagnostics based on quiet mode.
fn filter_diagnostics(diagnostics: &DiagnosticCollection, quiet: bool) -> Vec<Diagnostic> {
    // return only errors when quiet mode is enabled
    if quiet {
        diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .cloned()
            .collect()
    } else {
        diagnostics.to_vec()
    }
}

/// Count diagnostics by severity.
fn count_severity(diagnostics: &[Diagnostic]) -> (usize, usize, usize) {
    // compute aggregated counts
    let counts = count_by_severity(diagnostics);

    // read each severity bucket
    let error_count = *counts.get(&DiagnosticSeverity::Error).unwrap_or(&0);
    let warning_count = *counts.get(&DiagnosticSeverity::Warning).unwrap_or(&0);
    let note_count = *counts.get(&DiagnosticSeverity::Note).unwrap_or(&0);

    // return the final tuple
    (error_count, warning_count, note_count)
}

/// Build the json payload for diagnostics output.
fn build_diagnostic_output<F>(
    file_for_id: &F,
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    note_count: usize,
    options: &FormatOptions,
) -> DiagnosticOutputJson
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    // map diagnostics into json entries
    let json_diagnostics: Vec<DiagnosticJson> = diagnostics
        .iter()
        .map(|d| {
            let primary = d.primary_label();
            let primary_span = primary.span;
            let file = resolve_label_file(file_for_id, primary);

            // get_position returns 0 based line and column
            let (line, column) = file
                .get_position(primary_span.start)
                .map(|(l, c)| (l + 1, c + 1))
                .unwrap_or((1, 1));
            let (end_line, end_column) = file
                .get_position(primary_span.end)
                .map(|(l, c)| (l + 1, c + 1))
                .unwrap_or((1, 1));

            // resolve the file label
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

    // include statistics when requested
    let statistics = if options.statistics {
        Some(compute_statistics(diagnostics))
    } else {
        None
    };

    // build the output payload
    DiagnosticOutputJson {
        diagnostics: json_diagnostics,
        summary: DiagnosticSummaryJson {
            errors: error_count,
            warnings: warning_count,
            notes: note_count,
        },
        statistics,
    }
}

/// Resolve one file for one diagnostic label.
fn resolve_label_file<F>(file_for_id: &F, label: &DiagnosticLabel) -> Arc<File>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let file_id = label.span.file;
    let file =
        file_for_id(file_id).unwrap_or_else(|| panic!("missing diagnostic file: {file_id:?}"));
    let content = file.content_id();
    assert_eq!(
        content,
        label.content,
        "diagnostic content mismatch for file {file_id:?}: expected {expected}, got {content}",
        expected = label.content,
    );

    file
}

/// Compute statistics grouped by rule code.
fn compute_statistics(diagnostics: &[Diagnostic]) -> Vec<DiagnosticStatistic> {
    // aggregate counts per rule code
    let mut by_code: BTreeMap<String, (usize, DiagnosticSeverity)> = BTreeMap::new();
    for diagnostic in diagnostics {
        let entry = by_code
            .entry(diagnostic.code.clone())
            .or_insert((0, diagnostic.severity));
        entry.0 += 1;
    }

    // map the counts into statistics entries
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

    // return the ordered statistics
    stats
}

/// Count diagnostics by severity.
fn count_by_severity(diagnostics: &[Diagnostic]) -> BTreeMap<DiagnosticSeverity, usize> {
    // aggregate counts per severity
    let mut counts: BTreeMap<DiagnosticSeverity, usize> = BTreeMap::new();
    for d in diagnostics {
        *counts.entry(d.severity).or_insert(0) += 1;
    }

    // return the counts map
    counts
}

/// Write a line using the optional writer.
fn write_line(line_writer: Option<&LineWriter>, line: &str) {
    // use the writer when provided
    if let Some(writer) = line_writer {
        writer(line);
    } else {
        eprintln!("{line}");
    }
}
