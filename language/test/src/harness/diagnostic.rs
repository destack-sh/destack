use std::fmt::Write;

use destack_base::pluralize;
use destack_parser::source_colorizer;
use destack_source::{
    AnnotateOptions, DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, FileRegistry,
    PrintOptions, annotate_file,
};

use super::{TestCase, TestResult};

/// Format diagnostics with source annotations for display.
pub fn format_diagnostics(
    files: &FileRegistry,
    diagnostics: &DiagnosticCollection,
    options: PrintOptions,
) -> String {
    let mut annotate_options = AnnotateOptions::default().with_line_width(options.line_width);
    if let Some(colorizer) = options.colorizer.clone() {
        annotate_options = annotate_options.with_colorizer(colorizer);
    }

    let mut output = String::new();

    // individual diagnostics
    for diagnostic in diagnostics.iter() {
        let file = files.get(diagnostic.file_id);
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
        let header = {
            // include original code and severity if it exists and differs
            if let Some(original_code) = &diagnostic.original_code
                && let Some(original_severity) = diagnostic.original_severity
                && (*original_code != diagnostic.code || original_severity != diagnostic.severity)
            {
                let original_options = annotate_options
                    .clone()
                    .with_highlight_color(original_severity.color());
                let header_preamble_original = original_options
                    .color_highlight
                    .apply_bold(&original_code.to_string());
                let header_preamble = format!("{header_preamble} ({header_preamble_original})");
                format!("{header_preamble}: {header_message}")
            } else {
                format!("{header_preamble}: {header_message}")
            }
        };

        let body = annotate_file(&file, &diagnostic.primary_span, annotate_options);

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

/// Check diagnostics against test case expectations.
pub fn check_diagnostics(
    test: &TestCase,
    files: &FileRegistry,
    diagnostics: &DiagnosticCollector,
) -> TestResult {
    let all = diagnostics.collect().iter();

    // filter to unexpected diagnostics (at or above min_fail_severity)
    let unexpected_diagnostics: Vec<_> = all
        .into_iter()
        .filter(|d| severity_at_or_above(d.severity, test.min_fail_severity))
        .collect();

    if unexpected_diagnostics.is_empty() {
        return TestResult::Passed;
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
    let rendered = format_diagnostics(files, &unexpected_collection, options);

    TestResult::Failed {
        message: format!(
            "{}\n\n{}",
            parts.join(", "),
            rendered.trim_end_matches('\n')
        ),
    }
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
