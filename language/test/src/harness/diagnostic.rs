//! Diagnostic checking utilities for tests.

use destack_parser::source_colorizer;
use destack_source::{
    DiagnosticCollector, DiagnosticSeverity, FileRegistry, PrintOptions, pluralize,
    print_diagnostics,
};

use super::{TestCase, TestResult};

/// Check diagnostics against test case expectations.
pub fn check_diagnostics(
    test: &TestCase,
    files: &FileRegistry,
    diagnostics: &DiagnosticCollector,
) -> TestResult {
    let collection = diagnostics.collect();
    let all = collection.iter();

    // filter to unexpected diagnostics (at or above min_fail_severity)
    let unexpected: Vec<_> = all
        .iter()
        .filter(|d| severity_at_or_above(d.severity, test.min_fail_severity))
        .collect();

    if unexpected.is_empty() {
        return TestResult::Passed;
    }

    // print diagnostics
    let options = PrintOptions::new().with_colorizer(source_colorizer());
    print_diagnostics(files, &collection, options);

    // count by severity
    let mut error_count = 0;
    let mut warning_count = 0;
    let mut note_count = 0;
    for diag in &unexpected {
        match diag.severity {
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

    TestResult::Failed {
        message: parts.join(", "),
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
