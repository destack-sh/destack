use destack_workspace::query;
use destack_workspace::query::FoldingRange;

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{max_line_index, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a folding_ranges test.
///
/// Tests that the file produces the expected number of folding ranges.
/// Format: `query folding_ranges` with content showing expected count.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no folding_ranges expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // run the folding range query for the primary file
    let ranges = query::folding_ranges(&session.session, session.file_id);
    let content = exp.content.trim();

    // empty expectation is an error: must specify expected count
    if content.is_empty() {
        return TestResult::Failed {
            message: format!(
                "folding_ranges expectation is empty, got {} ranges",
                ranges.len()
            ),
        };
    }

    // validate folding range invariants before comparisons
    if let Err(message) = validate_folding_invariants(session, &ranges) {
        return TestResult::Failed { message };
    }

    // compare against a protocol shaped snapshot when structured
    if is_snapshot_expectation(content) {
        let actual_snapshot = normalize_expected_snapshot(&folding_snapshot(&ranges).join("\n"));
        let expected_snapshot = normalize_expected_snapshot(content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "folding_ranges snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
                ),
            };
        }

        return TestResult::Passed;
    }

    // parse expected count from content
    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("folding_ranges expectation '{content}' is not a valid count"),
        };
    };

    if ranges.len() != expected_count {
        TestResult::Failed {
            message: format!(
                "folding_ranges returned {} ranges, expected {expected_count}",
                ranges.len(),
            ),
        }
    } else {
        TestResult::Passed
    }
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    // detect structured snapshots by line range markers
    expected.lines().map(str::trim).any(|line| {
        let has_range = line.contains('-');
        let has_digits = line.chars().any(|ch| ch.is_ascii_digit());
        has_range && has_digits
    })
}

/// Validate folding range invariants.
fn validate_folding_invariants(
    session: &QueryTestSession,
    ranges: &[FoldingRange],
) -> Result<(), String> {
    let mut errors = Vec::new();
    let source = source_for_file(session, session.file_id);
    let max_line = max_line_index(source);

    // validate range bounds against the source
    for range in ranges {
        // ensure the start line does not exceed the end line
        if range.start_line > range.end_line {
            errors.push(format!(
                "folding range start {} is after end {}",
                range.start_line, range.end_line
            ));
        }

        // ensure the start line stays within the source bounds
        if range.start_line > max_line {
            errors.push(format!(
                "folding range start {} exceeds max line {}",
                range.start_line, max_line,
            ));
        }

        // ensure the end line stays within the source bounds
        if range.end_line > max_line {
            errors.push(format!(
                "folding range end {} exceeds max line {}",
                range.end_line, max_line
            ));
        }
    }

    // validate ranges are sorted and unique
    let mut previous: Option<(u32, u32)> = None;
    for range in ranges {
        let key = (range.start_line, range.end_line);

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = previous {
            if key < prev {
                errors.push(format!(
                    "folding ranges are not sorted: {:?} before {:?}",
                    prev, key
                ));
            }

            if key == prev {
                errors.push(format!("duplicate folding range {:?}", key));
            }
        }
        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "folding_ranges invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Format folding ranges as a 1 based line snapshot.
fn folding_snapshot(ranges: &[FoldingRange]) -> Vec<String> {
    let mut lines = Vec::new();
    for range in ranges {
        let start = range.start_line.saturating_add(1);
        let end = range.end_line.saturating_add(1);
        lines.push(format!("{start}-{end}"));
    }
    lines
}
