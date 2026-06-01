use destack_query::{InlayHint, InlayHintKind};
use destack_source::{FileId, Span};

use crate::core::CaseResult;
use crate::query::runner::snapshot::{
    compare_snapshot, looks_like_span_snapshot, normalize_expected_snapshot,
};
use crate::query::runner::span::{
    compute_line_starts, file_for, offset_to_line_col, source_for_file,
};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an inlay_hints test.
///
/// Verifies that inlay hints are shown at expected positions.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no inlay_hints expectation provided".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let range = Span::new(session.file_id, 0, session.source.len() as u32);
        let ctx = session.primary_module_context();
        let hints = ctx.inlay_hints(range);
        return CaseResult::Failed {
            message: format!(
                "inlay_hints expectation is empty, but query returned {} hints",
                hints.len()
            ),
        };
    }

    // get all hints for the file
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let ctx = session.primary_module_context();
    let hints = ctx.inlay_hints(range);

    // validate hint invariants before comparisons
    if let Err(message) = validate_inlay_hint_invariants(session, session.file_id, &hints) {
        return CaseResult::Failed { message };
    }

    // treat <none> as an explicit empty result expectation
    if content == "<none>" {
        if hints.is_empty() {
            return CaseResult::Passed;
        }

        let actual_snapshot = normalize_expected_snapshot(
            &inlay_hint_snapshot(session, session.file_id, &hints).join("\n"),
        );
        return CaseResult::Failed {
            message: format!("inlay_hints expected none, got:\n{actual_snapshot}"),
        };
    }

    // compare against a protocol shaped snapshot when structured
    if looks_like_span_snapshot(content, &["kind="]) {
        let actual_snapshot = inlay_hint_snapshot(session, session.file_id, &hints).join("\n");
        return compare_snapshot("inlay_hints", &actual_snapshot, content);
    }

    CaseResult::Failed {
        message: format!("inlay_hints requires an explicit snapshot or <none>, got '{content}'"),
    }
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
