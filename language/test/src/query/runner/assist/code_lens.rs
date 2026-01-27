use destack_source::Span;
use destack_workspace::query;
use destack_workspace::query::{CodeLens, CodeLensData};

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a code_lens test.
///
/// The expectation format is a list of lenses, one per line:
/// ```text
/// <lens_title>
/// ```
///
/// For example:
/// ```text
/// 2 references
/// 1 implementation
/// ▶ Run test_foo
/// ```
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no code_lens expectation defined".to_string(),
        };
    };

    // run the code lens query once for the primary file
    let lenses = query::code_lenses(&session.session, session.file_id);

    run_with_expectation(session, exp, &lenses)
}

/// Run with markdown expectation.
fn run_with_expectation(
    session: &QueryTestSession,
    exp: &QueryExpectation,
    lenses: &[CodeLens],
) -> TestResult {
    // normalize the expectation content
    let content = exp.content.trim();

    // empty expectations are not allowed
    if content.is_empty() {
        let actual_snapshot = format_lens_snapshot(session, lenses).join("\n");
        return TestResult::Failed {
            message: format!("code_lens expectation is empty, query returned:\n{actual_snapshot}"),
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_lens_invariants(session, lenses) {
        return TestResult::Failed { message };
    }

    // "<none>" means we expect no lenses
    if content == "<none>" {
        if lenses.is_empty() {
            return TestResult::Passed;
        }

        let actual_snapshot = format_lens_snapshot(session, lenses).join("\n");
        return TestResult::Failed {
            message: format!("code_lens expected no lenses, got:\n{actual_snapshot}"),
        };
    }

    // compare against protocol shaped snapshots when structured
    if is_snapshot_expectation(content) {
        let actual_snapshot =
            normalize_expected_snapshot(&format_lens_snapshot(session, lenses).join("\n"));
        let expected_snapshot = normalize_expected_snapshot(content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "code_lens snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
                ),
            };
        }

        return TestResult::Passed;
    }

    // parse expected lens titles
    let expected: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    // get actual lens titles
    let actual: Vec<String> = lenses.iter().map(|l| l.title()).collect();

    // compare counts
    if actual.len() != expected.len() {
        let actual_snapshot = format_lens_snapshot(session, lenses).join("\n");
        return TestResult::Failed {
            message: format!(
                "code_lenses count mismatch: expected {}, got {}\nExpected:\n{}\nActual:\n{}",
                expected.len(),
                actual.len(),
                expected.join("\n"),
                actual_snapshot
            ),
        };
    }

    // compare each lens title
    for (i, (exp_title, act_title)) in expected.iter().zip(actual.iter()).enumerate() {
        if *exp_title != act_title {
            return TestResult::Failed {
                message: format!("lens {i} mismatch: expected '{exp_title}', got '{act_title}'"),
            };
        }
    }

    TestResult::Passed
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    // detect structured snapshots by kind markers or span like digits
    expected.lines().map(str::trim).any(|line| {
        let has_kind_marker = line.contains("kind=") || line.contains("title=");
        let has_digit_span = line.contains(':') && line.chars().any(|c| c.is_ascii_digit());
        has_kind_marker || has_digit_span
    })
}

/// Validate code lens invariants.
fn validate_lens_invariants(session: &QueryTestSession, lenses: &[CodeLens]) -> Result<(), String> {
    let mut errors = Vec::new();
    let source = source_for_file(session, session.file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // validate lens ranges against the source bounds
    for lens in lenses {
        validate_span_bounds("code_lens", lens.range, source_len, &mut errors);
    }

    // validate ordering and duplicates
    let mut previous: Option<(u32, u32, u8, String)> = None;
    for lens in lenses {
        // build a stable ordering key for the lens
        let key = lens_key(lens);

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = &previous {
            if key < *prev {
                errors.push(format!(
                    "code_lens results are not sorted: {:?} before {:?}",
                    prev, key
                ));
            }

            if key == *prev {
                errors.push(format!("duplicate code_lens entry {:?}", key));
            }
        }

        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "code_lens invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate that a span is within the source bounds.
fn validate_span_bounds(label: &str, span: Span, source_len: u32, errors: &mut Vec<String>) {
    // ensure the span start does not exceed the end
    if span.start > span.end {
        errors.push(format!(
            "{label}: span start {} is after end {}",
            span.start, span.end
        ));
    }

    // ensure the span end stays within the source bounds
    if span.end > source_len {
        errors.push(format!(
            "{label}: span end {} exceeds source length {}",
            span.end, source_len
        ));
    }
}

/// Format code lenses as a protocol shaped snapshot.
fn format_lens_snapshot(session: &QueryTestSession, lenses: &[CodeLens]) -> Vec<String> {
    let mut lines = Vec::new();

    // render each lens with its range, kind, and title
    for lens in lenses {
        lines.push(format_lens_line(session, lens));
    }

    lines
}

/// Format a single code lens snapshot line.
fn format_lens_line(session: &QueryTestSession, lens: &CodeLens) -> String {
    let range = format_span_for_session(session, lens.range);
    let kind = lens_kind_name(&lens.data);
    let title = lens.title();

    format!("{range} kind={kind} title={title}")
}

/// Build a stable ordering key for a code lens.
fn lens_key(lens: &CodeLens) -> (u32, u32, u8, String) {
    let title = lens.title();
    (
        lens.range.start,
        lens.range.end,
        lens_kind_rank(&lens.data),
        title,
    )
}

/// Convert a code lens kind into a snapshot friendly name.
fn lens_kind_name(data: &CodeLensData) -> &'static str {
    match data {
        CodeLensData::References { .. } => "references",
        CodeLensData::Implementations { .. } => "implementations",
        CodeLensData::RunTest { .. } => "run_test",
        CodeLensData::DebugTest { .. } => "debug_test",
        CodeLensData::Custom { .. } => "custom",
    }
}

/// Rank code lens kinds for stable ordering checks.
fn lens_kind_rank(data: &CodeLensData) -> u8 {
    match data {
        CodeLensData::References { .. } => 0,
        CodeLensData::Implementations { .. } => 1,
        CodeLensData::RunTest { .. } => 2,
        CodeLensData::DebugTest { .. } => 3,
        CodeLensData::Custom { .. } => 4,
    }
}
