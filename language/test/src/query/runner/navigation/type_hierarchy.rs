use destack_query as query;
use destack_query::{TypeHierarchyItem, TypeHierarchyKind};
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{compare_snapshot, looks_like_span_snapshot};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a type_hierarchy test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no type_hierarchy expectation provided".to_string(),
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
        .unwrap_or("supertypes");

    // prepare type hierarchy item
    let Some(item) =
        query::prepare_type_hierarchy(&session.repository, session.revision, file_id, offset)
    else {
        return CaseResult::Failed {
            message: format!("type_hierarchy at '{}' returned None", exp.target),
        };
    };

    // validate the prepared item before expansion
    if let Err(message) = validate_item_invariants(session, &item) {
        return CaseResult::Failed { message };
    }

    // compute hierarchy items for the requested direction
    let (items, label) = match direction {
        "supertypes" | "super" => (
            query::supertypes(&session.repository, session.revision, &item),
            "supertypes",
        ),
        "subtypes" | "sub" => (
            query::subtypes(&session.repository, session.revision, &item),
            "subtypes",
        ),
        _ => {
            return CaseResult::Failed {
                message: format!(
                    "type_hierarchy direction '{direction}' is invalid, expected supertypes or subtypes"
                ),
            };
        }
    };

    // normalize the expected content for comparison
    let expected = exp.content.trim();

    // empty expectation is an error
    if expected.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "type_hierarchy {label} expectation is empty at '{}', got {} items",
                exp.target,
                items.len()
            ),
        };
    }

    // validate returned items before comparisons
    if let Err(message) = validate_items_invariants(session, &items, label) {
        return CaseResult::Failed { message };
    }

    // "<none>" means we expect no items
    if expected == "<none>" {
        if items.is_empty() {
            return CaseResult::Passed;
        }
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        return CaseResult::Failed {
            message: format!("type_hierarchy {label} expected no results, got {names:?}"),
        };
    }

    // compare against protocol shaped snapshots when structured
    if looks_like_span_snapshot(expected, &["kind=", "selection=", "detail="]) {
        return compare_snapshot(
            &format!("type_hierarchy {label} at '{}'", exp.target),
            &snapshot_items(session, &items),
            expected,
        );
    }

    // parse expected names
    let expected_names: Vec<&str> = expected
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    let actual_names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();

    // compare counts
    if expected_names.len() != actual_names.len() {
        return CaseResult::Failed {
            message: format!(
                "type_hierarchy {label} count mismatch: expected {}, got {} ({actual_names:?})",
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
    let mut actual_names = actual_names
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    expected_names.sort();
    actual_names.sort();

    if actual_names != expected_names {
        return CaseResult::Failed {
            message: format!(
                "type_hierarchy {label} names mismatch: expected {expected_names:?}, got {actual_names:?}"
            ),
        };
    }

    CaseResult::Passed
}

/// Decide whether an expectation is a structured snapshot.
/// Format type hierarchy items into a protocol shaped snapshot.
fn snapshot_items(session: &QueryTestSession, items: &[TypeHierarchyItem]) -> String {
    // format each item into a single snapshot line
    items
        .iter()
        .map(|item| format_item_line(session, item))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a single type hierarchy item snapshot line.
fn format_item_line(session: &QueryTestSession, item: &TypeHierarchyItem) -> String {
    let range = format_span_for_session(session, item.range);
    let selection = format_span_for_session(session, item.selection_range);
    let kind = type_hierarchy_kind_name(item.kind);

    // include detail when it is present
    if let Some(detail) = &item.detail {
        return format!(
            "{range} name={} kind={kind} selection={selection} detail={detail}",
            item.name
        );
    }

    format!(
        "{range} name={} kind={kind} selection={selection}",
        item.name
    )
}

/// Convert a type hierarchy kind into a snapshot friendly name.
fn type_hierarchy_kind_name(kind: TypeHierarchyKind) -> &'static str {
    match kind {
        TypeHierarchyKind::Class => "class",
        TypeHierarchyKind::Interface => "interface",
        TypeHierarchyKind::Struct => "struct",
        TypeHierarchyKind::Enum => "enum",
        TypeHierarchyKind::TypeAlias => "type_alias",
    }
}

/// Rank type hierarchy kinds for stable ordering checks.
fn type_hierarchy_kind_rank(kind: TypeHierarchyKind) -> u8 {
    match kind {
        TypeHierarchyKind::Class => 0,
        TypeHierarchyKind::Interface => 1,
        TypeHierarchyKind::Struct => 2,
        TypeHierarchyKind::Enum => 3,
        TypeHierarchyKind::TypeAlias => 4,
    }
}

/// Validate a single type hierarchy item.
fn validate_item_invariants(
    session: &QueryTestSession,
    item: &TypeHierarchyItem,
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate the item spans against the source bounds
    validate_item_collect_errors(session, item, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "type_hierarchy item invariant violations for '{}':\n{}",
            item.name,
            errors.join("\n")
        ))
    }
}

/// Validate a list of type hierarchy items and their ordering.
fn validate_items_invariants(
    session: &QueryTestSession,
    items: &[TypeHierarchyItem],
    label: &str,
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate each item against the source bounds
    for item in items {
        validate_item_collect_errors(session, item, &mut errors);
    }

    // validate ordering and duplicates
    let mut previous: Option<(u128, u32, u32, u32, u32, u8, String)> = None;
    for item in items {
        // build a stable ordering key for the item
        let key = item_key(item);

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = &previous {
            if key < *prev {
                errors.push(format!(
                    "type_hierarchy {label} items are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == *prev {
                errors.push(format!("duplicate type_hierarchy {label} item {key:?}"));
            }
        }

        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "type_hierarchy {label} invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate item spans and containment, collecting errors.
fn validate_item_collect_errors(
    session: &QueryTestSession,
    item: &TypeHierarchyItem,
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

/// Build a stable ordering key for a type hierarchy item.
fn item_key(item: &TypeHierarchyItem) -> (u128, u32, u32, u32, u32, u8, String) {
    (
        item.file.0,
        item.range.start,
        item.range.end,
        item.selection_range.start,
        item.selection_range.end,
        type_hierarchy_kind_rank(item.kind),
        item.name.clone(),
    )
}
