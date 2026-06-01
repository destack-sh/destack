use destack_query::SelectionRange;
use destack_source::{FileId, Span};

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{compare_snapshot, looks_like_span_snapshot};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a selection_range test.
///
/// Tests that selecting at a position produces the expected selection chain.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no selection_range expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    let ctx = session.module_context(file_id);
    let ranges = ctx.selection_ranges(&[offset]);

    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let depth = ranges.first().map(|r| r.depth()).unwrap_or(0);
        return CaseResult::Failed {
            message: format!("selection_range expectation is empty, got depth {depth}"),
        };
    }

    let Some(selection) = ranges.first() else {
        return CaseResult::Failed {
            message: "selection_range returned no ranges".to_string(),
        };
    };

    // flatten the selection chain from leaf to root
    let chain = selection_chain(selection);

    // validate selection invariants before comparisons
    if let Err(message) = validate_selection_invariants(session, file_id, &chain) {
        return CaseResult::Failed { message };
    }

    // compare against a protocol shaped snapshot when structured
    if looks_like_span_snapshot(content, &["range="]) {
        let actual_snapshot = chain
            .iter()
            .map(|span| format_span_for_session(session, *span))
            .collect::<Vec<_>>()
            .join("\n");
        return compare_snapshot(
            &format!("selection_range at '{}'", exp.target),
            &actual_snapshot,
            content,
        );
    }

    CaseResult::Failed {
        message: format!(
            "selection_range at '{}' requires an explicit span snapshot, got '{content}'",
            exp.target
        ),
    }
}

/// Decide whether an expectation is a structured snapshot.
/// Flatten a selection range chain from leaf to root.
fn selection_chain(selection: &SelectionRange) -> Vec<Span> {
    // collect the selection chain from leaf to root
    let mut chain = Vec::new();
    let mut current = selection;

    // walk parents until we reach the root selection
    loop {
        chain.push(current.range);
        let Some(parent) = &current.parent else {
            break;
        };
        current = parent;
    }

    chain
}

/// Validate selection range invariants.
fn validate_selection_invariants(
    session: &QueryTestSession,
    file_id: FileId,
    chain: &[Span],
) -> Result<(), String> {
    let mut errors = Vec::new();
    let source = source_for_file(session, file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // validate basic span bounds
    for span in chain {
        // ensure the span start does not exceed the end
        if span.start > span.end {
            errors.push(format!(
                "selection span start {} is after end {}",
                span.start, span.end
            ));
        }

        // ensure the span end stays within the source bounds
        if span.end > source_len {
            errors.push(format!(
                "selection span end {} exceeds source length {}",
                span.end, source_len
            ));
        }
    }

    // validate monotonic containment from leaf to root
    for window in chain.windows(2) {
        let child = window[0];
        let parent = window[1];

        // ensure each parent fully contains its child selection
        let contains = parent.start <= child.start && parent.end >= child.end;
        if !contains {
            errors.push(format!(
                "parent {parent:?} does not contain child {child:?}"
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "selection_range invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}
