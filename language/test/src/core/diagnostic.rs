use std::fmt::Write;
use std::sync::Arc;

use destack_core::pluralize;
use destack_parser::source_colorizer;
use destack_repository::{Repository, Revision};
use destack_source::{
    AnnotateOptions, DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, File, FileId,
    PrintOptions, annotate_file,
};

use super::{Case, CaseResult};

/// Format diagnostics with source annotations for display.
pub fn format_diagnostics<F>(
    file_for_id: &F,
    diagnostics: &DiagnosticCollection,
    options: PrintOptions,
) -> String
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let mut annotate_options = AnnotateOptions::default().with_line_width(options.line_width);
    if let Some(colorizer) = options.colorizer.clone() {
        annotate_options = annotate_options.with_colorizer(colorizer);
    }

    let mut output = String::new();

    // individual diagnostics
    for diagnostic in diagnostics.iter() {
        let primary = diagnostic.primary_label();
        let file_id = primary.span.file;
        let file =
            file_for_id(file_id).unwrap_or_else(|| panic!("missing diagnostic file: {file_id:?}"));
        let annotate_options = annotate_options
            .clone()
            .with_highlight_color(diagnostic.severity.color());

        // header preamble (severity + code)
        let header_preamble = annotate_options.color_highlight.apply_bold(&format!(
            "{} {}",
            diagnostic.severity.family_name().to_ascii_lowercase(),
            diagnostic.code,
        ));

        let header_message = annotate_options.color_normal.apply(&diagnostic.message);
        let header = format!("{header_preamble}: {header_message}");
        let primary = primary.to_labeled_span(&diagnostic.message);
        let body = annotate_file(&file, &primary, annotate_options)
            .unwrap_or_else(|error| panic!("failed to render diagnostic: {error}"));

        let _ = writeln!(output, "{header}");
        let _ = writeln!(output, "{body}");
    }

    // summary
    let counts = diagnostics.count_diagnostics_by_severity();
    if !counts.is_empty() {
        // derive families and pluralize with naive 's'
        let mut parts: Vec<String> = Vec::new();
        for (severity, count) in counts.iter().rev() {
            let name = severity.family_name().to_ascii_lowercase();
            let color = severity.color();
            let colored_count = color.apply_bold(&format!("{count}"));
            let colored_name = color.apply_bold(&pluralize(*count, name));
            parts.push(format!("{colored_count} {colored_name}"));
        }
        let summary = parts.join(", ");

        // summary line
        let highest_severity = counts.keys().max().unwrap();
        let color = highest_severity.color();
        let name = highest_severity.family_name().to_ascii_lowercase();

        if let Some(module_count) = options.module_count {
            let _ = writeln!(
                output,
                "{}: {} from {} {}",
                color.apply_bold(&name),
                summary,
                module_count,
                pluralize(module_count, "module")
            );
        } else {
            let _ = writeln!(output, "{}: {}", color.apply_bold(&name), summary);
        }
    }

    output
}

/// Check diagnostics against one case expectation.
pub fn check_diagnostics(
    case: &Case,
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollector,
) -> CaseResult {
    let Some(message) =
        render_unexpected_diagnostics(file_for_id, diagnostics, case.min_fail_severity)
    else {
        return CaseResult::Passed;
    };

    CaseResult::Failed { message }
}

/// Check collected diagnostics against one case expectation.
pub fn check_diagnostic_collection(
    case: &Case,
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollection,
) -> CaseResult {
    let Some(message) =
        render_unexpected_diagnostic_collection(file_for_id, diagnostics, case.min_fail_severity)
    else {
        return CaseResult::Passed;
    };

    CaseResult::Failed { message }
}

/// Check diagnostics against one case using one repository snapshot.
pub fn check_repository_diagnostics(
    case: &Case,
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollector,
) -> CaseResult {
    let file_for_id = |file_id| {
        repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load diagnostic file {file_id:?}: {error}"))
    };

    check_diagnostics(case, &file_for_id, diagnostics)
}

/// Check collected diagnostics against one case using one repository snapshot.
pub fn check_repository_diagnostic_collection(
    case: &Case,
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> CaseResult {
    let file_for_id = |file_id| {
        repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load diagnostic file {file_id:?}: {error}"))
    };

    check_diagnostic_collection(case, &file_for_id, diagnostics)
}

/// Render all unexpected diagnostics at or above one minimum severity.
pub fn render_unexpected_diagnostics(
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollector,
    min_fail_severity: DiagnosticSeverity,
) -> Option<String> {
    render_unexpected_diagnostic_collection(file_for_id, &diagnostics.collect(), min_fail_severity)
}

/// Render unexpected collected diagnostics at or above one minimum severity.
pub fn render_unexpected_diagnostic_collection(
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollection,
    min_fail_severity: DiagnosticSeverity,
) -> Option<String> {
    // filter to unexpected diagnostics (at or above min_fail_severity)
    let unexpected_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|&d| severity_at_or_above(d.severity, min_fail_severity))
        .cloned()
        .collect();

    if unexpected_diagnostics.is_empty() {
        return None;
    }

    // format unexpected diagnostics for deterministic output ordering
    let mut unexpected_collection = DiagnosticCollection::new();
    for diagnostic in unexpected_diagnostics.iter().cloned() {
        unexpected_collection.insert(diagnostic);
    }

    // count by severity
    let mut error_count = 0;
    let mut warning_count = 0;
    let mut note_count = 0;
    for diagnostic in &unexpected_diagnostics {
        match diagnostic.severity {
            DiagnosticSeverity::Error => error_count += 1,
            DiagnosticSeverity::Warning => warning_count += 1,
            DiagnosticSeverity::Note => note_count += 1,
        }
    }

    // build message like "1 error, 2 warnings"
    let mut parts = Vec::new();
    if error_count > 0 {
        parts.push(format!("{error_count} {}", pluralize(error_count, "error")));
    }
    if warning_count > 0 {
        parts.push(format!(
            "{warning_count} {}",
            pluralize(warning_count, "warning")
        ));
    }
    if note_count > 0 {
        parts.push(format!("{note_count} {}", pluralize(note_count, "note")));
    }

    let options = PrintOptions::new().with_colorizer(source_colorizer());
    let rendered = format_diagnostics(file_for_id, &unexpected_collection, options);

    Some(format!(
        "{}\n\n{}",
        parts.join(", "),
        rendered.trim_end_matches('\n')
    ))
}

/// Render unexpected diagnostics using one repository snapshot.
pub fn render_unexpected_repository_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollector,
    min_fail_severity: DiagnosticSeverity,
) -> Option<String> {
    let file_for_id = |file_id| {
        repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load diagnostic file {file_id:?}: {error}"))
    };

    render_unexpected_diagnostics(&file_for_id, diagnostics, min_fail_severity)
}

/// Render unexpected collected diagnostics using one repository snapshot.
pub fn render_unexpected_repository_diagnostic_collection(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
    min_fail_severity: DiagnosticSeverity,
) -> Option<String> {
    let file_for_id = |file_id| {
        repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load diagnostic file {file_id:?}: {error}"))
    };

    render_unexpected_diagnostic_collection(&file_for_id, diagnostics, min_fail_severity)
}

/// Check if `actual` severity is at or above `threshold`.
fn severity_at_or_above(actual: DiagnosticSeverity, threshold: DiagnosticSeverity) -> bool {
    severity_level(actual) >= severity_level(threshold)
}

/// Convert severity to numeric level for comparison (higher = more severe).
fn severity_level(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Note => 1,
        DiagnosticSeverity::Warning => 2,
        DiagnosticSeverity::Error => 3,
    }
}
