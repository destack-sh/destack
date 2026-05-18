use destack_query as query;
use destack_query::FoldingRange;

use crate::core::CaseResult;
use crate::query::runner::snapshot::{compare_snapshot_lines, looks_like_span_snapshot};
use crate::query::runner::span::{max_line_index, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a folding_ranges test.
///
/// Tests that the file produces the exact expected folding ranges.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no folding_ranges expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // run the folding range query for the primary file
    let ctx = session.primary_module_context();
    let ranges = query::folding_ranges(&ctx);
    let content = exp.content.trim();

    // empty expectation is an error: must specify expected count
    if content.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "folding_ranges expectation is empty, got {} ranges",
                ranges.len()
            ),
        };
    }

    // validate folding range invariants before comparisons
    if let Err(message) = validate_folding_invariants(session, &ranges) {
        return CaseResult::Failed { message };
    }

    // compare against a protocol shaped snapshot when structured
    if looks_like_span_snapshot(content, &[]) {
        return compare_snapshot_lines("folding_ranges", &folding_snapshot(&ranges), content);
    }

    // treat <none> as an explicit empty result expectation
    if content == "<none>" {
        return if ranges.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!(
                    "folding_ranges expected none, got:\n{}",
                    folding_snapshot(&ranges).join("\n")
                ),
            }
        };
    }

    CaseResult::Failed {
        message: format!("folding_ranges requires an explicit range snapshot, got '{content}'",),
    }
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
                    "folding ranges are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == prev {
                errors.push(format!("duplicate folding range {key:?}"));
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
