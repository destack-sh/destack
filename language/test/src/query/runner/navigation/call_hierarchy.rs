use destack_query as query;
use destack_query::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyKind, CallHierarchyOutgoingCall,
};
use destack_source::{FileId, Span};

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{compare_snapshot, looks_like_span_snapshot};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a call_hierarchy test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no call_hierarchy expectation provided".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // parse direction from args
    let direction = exp
        .args
        .first()
        .map(|arg| arg.as_str())
        .unwrap_or("incoming");

    // compute calls for the requested direction
    let expected = exp.content.trim();

    // empty expectation is an error
    if expected.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "call_hierarchy {direction} expectation is empty at '{}'",
                exp.target
            ),
        };
    }

    // prepare call hierarchy item
    let item =
        query::prepare_call_hierarchy(&session.repository, session.revision, file_id, offset);

    // allow "<none>" to assert that no hierarchy item exists at all
    if expected == "<none>" && item.is_none() {
        return CaseResult::Passed;
    }

    // otherwise a hierarchy item must exist
    let Some(item) = item else {
        return CaseResult::Failed {
            message: format!("call_hierarchy at '{}' returned None", exp.target),
        };
    };

    // validate the prepared item before expansion
    if let Err(message) = validate_item_invariants(session, &item) {
        return CaseResult::Failed { message };
    }

    // dispatch by direction
    match direction {
        "incoming" => run_incoming_expectation(session, &item, expected, &exp.target),
        "outgoing" => run_outgoing_expectation(session, &item, expected, &exp.target),
        _ => CaseResult::Failed {
            message: format!(
                "call_hierarchy direction '{direction}' is invalid, expected incoming or outgoing"
            ),
        },
    }
}

/// Run incoming call hierarchy expectations.
fn run_incoming_expectation(
    session: &QueryTestSession,
    item: &CallHierarchyItem,
    expected: &str,
    target: &str,
) -> CaseResult {
    // run the incoming calls query
    let calls = query::incoming_calls(&session.repository, session.revision, item);

    // validate invariants before comparisons
    if let Err(message) = validate_incoming_invariants(session, &calls) {
        return CaseResult::Failed { message };
    }

    // "<none>" means we expect no calls
    if expected == "<none>" {
        if calls.is_empty() {
            return CaseResult::Passed;
        }
        let actual_names: Vec<&str> = calls.iter().map(|call| call.from.name.as_str()).collect();
        return CaseResult::Failed {
            message: format!("call_hierarchy incoming expected no results, got {actual_names:?}"),
        };
    }

    // compare against protocol shaped snapshots when structured
    if looks_like_span_snapshot(expected, &["kind=", "selection=", "calls="]) {
        return compare_snapshot(
            &format!("call_hierarchy incoming at '{target}'"),
            &snapshot_incoming(session, &calls),
            expected,
        );
    }

    // parse expected names
    let expected_names: Vec<&str> = expected
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    // collect actual names for comparison
    let actual_names: Vec<String> = calls.iter().map(|call| call.from.name.clone()).collect();

    // compare counts
    if expected_names.len() != actual_names.len() {
        return CaseResult::Failed {
            message: format!(
                "call_hierarchy incoming count mismatch: expected {}, got {} ({actual_names:?})",
                expected_names.len(),
                actual_names.len()
            ),
        };
    }

    // compare the exact multiset of names
    let mut expected_names = expected_names
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut actual_names = actual_names;
    expected_names.sort();
    actual_names.sort();

    if actual_names != expected_names {
        return CaseResult::Failed {
            message: format!(
                "call_hierarchy incoming names mismatch: expected {expected_names:?}, got {actual_names:?}"
            ),
        };
    }

    CaseResult::Passed
}

/// Run outgoing call hierarchy expectations.
fn run_outgoing_expectation(
    session: &QueryTestSession,
    item: &CallHierarchyItem,
    expected: &str,
    target: &str,
) -> CaseResult {
    // run the outgoing calls query
    let calls = query::outgoing_calls(&session.repository, session.revision, item);

    // validate invariants before comparisons
    if let Err(message) = validate_outgoing_invariants(session, item, &calls) {
        return CaseResult::Failed { message };
    }

    // "<none>" means we expect no calls
    if expected == "<none>" {
        if calls.is_empty() {
            return CaseResult::Passed;
        }
        let actual_names: Vec<&str> = calls.iter().map(|call| call.to.name.as_str()).collect();
        return CaseResult::Failed {
            message: format!("call_hierarchy outgoing expected no results, got {actual_names:?}"),
        };
    }

    // compare against protocol shaped snapshots when structured
    if looks_like_span_snapshot(expected, &["kind=", "selection=", "calls="]) {
        return compare_snapshot(
            &format!("call_hierarchy outgoing at '{target}'"),
            &snapshot_outgoing(session, &calls),
            expected,
        );
    }

    // parse expected names
    let expected_names: Vec<&str> = expected
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    // collect actual names for comparison
    let actual_names: Vec<String> = calls.iter().map(|call| call.to.name.clone()).collect();

    // compare counts
    if expected_names.len() != actual_names.len() {
        return CaseResult::Failed {
            message: format!(
                "call_hierarchy outgoing count mismatch: expected {}, got {} ({actual_names:?})",
                expected_names.len(),
                actual_names.len()
            ),
        };
    }

    // compare the exact multiset of names
    let mut expected_names = expected_names
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut actual_names = actual_names;
    expected_names.sort();
    actual_names.sort();

    if actual_names != expected_names {
        return CaseResult::Failed {
            message: format!(
                "call_hierarchy outgoing names mismatch: expected {expected_names:?}, got {actual_names:?}"
            ),
        };
    }

    CaseResult::Passed
}

/// Format incoming calls into a protocol shaped snapshot.
fn snapshot_incoming(session: &QueryTestSession, calls: &[CallHierarchyIncomingCall]) -> String {
    // format each incoming call into a single snapshot line
    calls
        .iter()
        .map(|call| format_call_line(session, &call.from, &call.from_ranges))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format outgoing calls into a protocol shaped snapshot.
fn snapshot_outgoing(session: &QueryTestSession, calls: &[CallHierarchyOutgoingCall]) -> String {
    // format each outgoing call into a single snapshot line
    calls
        .iter()
        .map(|call| format_call_line(session, &call.to, &call.from_ranges))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a single call hierarchy line.
fn format_call_line(
    session: &QueryTestSession,
    item: &CallHierarchyItem,
    ranges: &[Span],
) -> String {
    let range = format_span_for_session(session, item.range);
    let selection = format_span_for_session(session, item.selection_range);
    let kind = call_kind_name(item.kind);
    let calls = format_call_ranges(session, ranges);

    format!(
        "{range} name={} kind={kind} selection={selection} calls={calls}",
        item.name
    )
}

/// Format call ranges into a compact list.
fn format_call_ranges(session: &QueryTestSession, ranges: &[Span]) -> String {
    // format ranges and join them with a pipe separator
    ranges
        .iter()
        .map(|span| format_span_for_session(session, *span))
        .collect::<Vec<_>>()
        .join("|")
}

/// Convert a call hierarchy kind into a snapshot friendly name.
fn call_kind_name(kind: CallHierarchyKind) -> &'static str {
    match kind {
        CallHierarchyKind::Function => "function",
        CallHierarchyKind::Method => "method",
        CallHierarchyKind::Constructor => "constructor",
    }
}

/// Rank call hierarchy kinds for stable ordering checks.
fn call_kind_rank(kind: CallHierarchyKind) -> u8 {
    match kind {
        CallHierarchyKind::Function => 0,
        CallHierarchyKind::Method => 1,
        CallHierarchyKind::Constructor => 2,
    }
}

/// Validate the prepared call hierarchy item.
fn validate_item_invariants(
    session: &QueryTestSession,
    item: &CallHierarchyItem,
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate the item spans against the source bounds
    validate_item_collect_errors(session, item, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "call_hierarchy item invariant violations for '{}':\n{}",
            item.name,
            errors.join("\n")
        ))
    }
}

/// Validate incoming call invariants.
fn validate_incoming_invariants(
    session: &QueryTestSession,
    calls: &[CallHierarchyIncomingCall],
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate each incoming caller and its call sites
    for call in calls {
        validate_item_collect_errors(session, &call.from, &mut errors);
        validate_call_ranges(
            session,
            call.from.file,
            &call.from.name,
            &call.from_ranges,
            &mut errors,
        );
    }

    // validate ordering and duplicates
    validate_call_ordering(calls.iter().map(|call| &call.from), &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "call_hierarchy incoming invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate outgoing call invariants.
fn validate_outgoing_invariants(
    session: &QueryTestSession,
    item: &CallHierarchyItem,
    calls: &[CallHierarchyOutgoingCall],
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate each outgoing callee and its call sites
    for call in calls {
        validate_item_collect_errors(session, &call.to, &mut errors);
        validate_call_ranges(
            session,
            item.file,
            &call.to.name,
            &call.from_ranges,
            &mut errors,
        );
    }

    // validate ordering and duplicates
    validate_call_ordering(calls.iter().map(|call| &call.to), &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "call_hierarchy outgoing invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate item spans and containment, collecting errors.
fn validate_item_collect_errors(
    session: &QueryTestSession,
    item: &CallHierarchyItem,
    errors: &mut Vec<String>,
) {
    let source = source_for_file(session, item.file);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // validate the item range bounds
    validate_span_bounds(&item.name, "range", item.range, source_len, errors);

    // validate the selection range bounds
    validate_span_bounds(
        &item.name,
        "selection",
        item.selection_range,
        source_len,
        errors,
    );

    // ensure the selection stays within the full range
    if item.selection_range.start < item.range.start || item.selection_range.end > item.range.end {
        errors.push(format!(
            "{}: selection {:?} is outside range {:?}",
            item.name, item.selection_range, item.range
        ));
    }
}

/// Validate call site ranges for a call hierarchy item.
fn validate_call_ranges(
    session: &QueryTestSession,
    file_id: FileId,
    name: &str,
    ranges: &[Span],
    errors: &mut Vec<String>,
) {
    let source = source_for_file(session, file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // validate each call site span against the source bounds
    for span in ranges {
        validate_span_bounds(name, "call", *span, source_len, errors);
    }

    // validate ordering and duplicates within the call site ranges
    let mut previous: Option<(u128, u32, u32)> = None;
    for span in ranges {
        // build a stable ordering key for the call site span
        let key = span_key(*span);

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = previous {
            if key < prev {
                errors.push(format!(
                    "{name}: call site spans are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == prev {
                errors.push(format!("{name}: duplicate call site span {key:?}"));
            }
        }

        previous = Some(key);
    }
}

/// Validate ordering and duplicates for call hierarchy items.
fn validate_call_ordering<'a>(
    items: impl Iterator<Item = &'a CallHierarchyItem>,
    errors: &mut Vec<String>,
) {
    let mut previous: Option<(u128, u32, u32, u32, u32, u8, String)> = None;

    // compare each item key against the previous one
    for item in items {
        let key = item_key(item);

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = &previous {
            if key < *prev {
                errors.push(format!(
                    "call_hierarchy items are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == *prev {
                errors.push(format!("duplicate call_hierarchy item {key:?}"));
            }
        }

        previous = Some(key);
    }
}

/// Validate that a span is within the source bounds.
fn validate_span_bounds(
    name: &str,
    label: &str,
    span: Span,
    source_len: u32,
    errors: &mut Vec<String>,
) {
    // ensure the span start does not exceed the end
    if span.start > span.end {
        errors.push(format!(
            "{name}: {label} start {} is after end {}",
            span.start, span.end
        ));
    }

    // ensure the span end stays within the source bounds
    if span.end > source_len {
        errors.push(format!(
            "{name}: {label} end {} exceeds source length {}",
            span.end, source_len
        ));
    }
}

/// Build a stable ordering key for a call hierarchy item.
fn item_key(item: &CallHierarchyItem) -> (u128, u32, u32, u32, u32, u8, String) {
    (
        item.file.0,
        item.range.start,
        item.range.end,
        item.selection_range.start,
        item.selection_range.end,
        call_kind_rank(item.kind),
        item.name.clone(),
    )
}

/// Build a stable ordering key for a span.
fn span_key(span: Span) -> (u128, u32, u32) {
    (span.file.0, span.start, span.end)
}
