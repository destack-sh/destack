use destack_source::{FileId, Span};
use destack_workspace::query;
use destack_workspace::query::{InlayHint, InlayHintKind};

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{
    compute_line_starts, file_for, offset_to_line_col, source_for_file,
};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an inlay_hints test.
///
/// Verifies that inlay hints are shown at expected positions.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check inlay_hint expectations from markers
    let expected_hints = &session.markers.expectations.inlay_hints;
    if expected_hints.is_empty() {
        return TestResult::Skipped {
            reason: "no inlay_hint expectations defined".to_string(),
        };
    }

    // get all hints for the file
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let hints = query::inlay_hints(&session.session, session.file_id, range);

    // validate hint invariants before marker comparisons
    if let Err(message) = validate_inlay_hint_invariants(session, session.file_id, &hints) {
        return TestResult::Failed { message };
    }

    // ensure each expected hint is present in the results
    for expected_hint in expected_hints {
        let found = hints
            .iter()
            .any(|h| h.position == expected_hint.offset && h.label.contains(&expected_hint.label));

        if !found {
            let actual: Vec<_> = hints
                .iter()
                .map(|h| format!("{}:{}", h.position, h.label))
                .collect();
            return TestResult::Failed {
                message: format!(
                    "inlay_hint at offset {} with label '{}' not found\nactual: {:?}",
                    expected_hint.offset, expected_hint.label, actual
                ),
            };
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let range = Span::new(session.file_id, 0, session.source.len() as u32);
        let hints = query::inlay_hints(&session.session, session.file_id, range);
        return TestResult::Failed {
            message: format!(
                "inlay_hints expectation is empty, but query returned {} hints",
                hints.len()
            ),
        };
    }

    // get all hints for the file
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let hints = query::inlay_hints(&session.session, session.file_id, range);

    // validate hint invariants before comparisons
    if let Err(message) = validate_inlay_hint_invariants(session, session.file_id, &hints) {
        return TestResult::Failed { message };
    }

    // treat <none> as an explicit empty result expectation
    if content == "<none>" {
        if hints.is_empty() {
            return TestResult::Passed;
        }

        let actual_snapshot = normalize_expected_snapshot(
            &inlay_hint_snapshot(session, session.file_id, &hints).join("\n"),
        );
        return TestResult::Failed {
            message: format!("inlay_hints expected none, got:\n{actual_snapshot}"),
        };
    }

    // compare against a protocol shaped snapshot when structured
    if is_snapshot_expectation(content) {
        let actual_snapshot = normalize_expected_snapshot(
            &inlay_hint_snapshot(session, session.file_id, &hints).join("\n"),
        );
        let expected_snapshot = normalize_expected_snapshot(content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "inlay_hints snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
                ),
            };
        }

        return TestResult::Passed;
    }

    // parse expected count from content
    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("inlay_hints expectation '{content}' is not a valid count"),
        };
    };

    if hints.len() != expected_count {
        TestResult::Failed {
            message: format!(
                "inlay_hints returned {} hints, expected {}",
                hints.len(),
                expected_count
            ),
        }
    } else {
        TestResult::Passed
    }
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    // detect structured snapshots by kind markers or line and column spans
    expected.lines().map(str::trim).any(|line| {
        let has_kind = line.contains("kind=");
        let has_span = line.contains(':') && line.chars().any(|ch| ch.is_ascii_digit());
        has_kind || has_span
    })
}

/// Validate inlay hint invariants.
fn validate_inlay_hint_invariants(
    session: &QueryTestSession,
    file_id: FileId,
    hints: &[InlayHint],
) -> Result<(), String> {
    let mut errors = Vec::new();

    // resolve the source bounds for this file
    let source = source_for_file(session, file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // validate positions and basic kind semantics
    for hint in hints {
        // ensure the position stays within the source bounds
        if hint.position > source_len {
            errors.push(format!(
                "inlay hint position {} exceeds source length {}",
                hint.position, source_len
            ));
        }

        // require non empty hint labels
        if hint.label.trim().is_empty() {
            errors.push("inlay hint label is empty".to_string());
        }

        // enforce kind specific label conventions
        match hint.kind {
            InlayHintKind::Type => {
                let is_type_label = hint.label.starts_with(": ");
                if !is_type_label {
                    errors.push(format!(
                        "type hint label '{}' is not type formatted",
                        hint.label
                    ));
                }
            }
            InlayHintKind::Parameter => {
                let has_suffix = hint.label.ends_with(':');
                if !has_suffix {
                    errors.push(format!(
                        "parameter hint label '{}' does not end with ':'",
                        hint.label
                    ));
                }

                let has_padding = hint.padding_right;
                if !has_padding {
                    errors.push("parameter hint is missing right padding".to_string());
                }
            }
        }
    }

    // validate ordering and duplicates
    let mut previous: Option<(u32, u8, &str)> = None;
    for hint in hints {
        // build a stable ordering key for the hint
        let key = (
            hint.position,
            hint_kind_rank(hint.kind),
            hint.label.as_str(),
        );

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = previous {
            if key < prev {
                errors.push(format!(
                    "inlay hints are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == prev {
                errors.push(format!("duplicate inlay hint {key:?}"));
            }
        }
        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "inlay_hints invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Format inlay hints as a protocol shaped snapshot.
fn inlay_hint_snapshot(
    session: &QueryTestSession,
    file_id: FileId,
    hints: &[InlayHint],
) -> Vec<String> {
    let source = source_for_file(session, file_id);
    let line_starts = compute_line_starts(source);
    let file_name = file_for(session, file_id)
        .map(|file| file.name.as_str())
        .unwrap_or("<unknown>");

    let mut lines = Vec::new();
    for hint in hints {
        let position = offset_to_line_col(&line_starts, hint.position);
        let kind = hint_kind_name(hint.kind);
        lines.push(format!(
            "{file_name}:{}:{} kind={kind} label={}",
            position.0, position.1, hint.label
        ));
    }
    lines
}

/// Rank inlay hint kinds for stable sorting in tests.
fn hint_kind_rank(kind: InlayHintKind) -> u8 {
    match kind {
        InlayHintKind::Type => 0,
        InlayHintKind::Parameter => 1,
    }
}

/// Format an inlay hint kind as a lowercase name.
fn hint_kind_name(kind: InlayHintKind) -> &'static str {
    match kind {
        InlayHintKind::Type => "type",
        InlayHintKind::Parameter => "parameter",
    }
}
