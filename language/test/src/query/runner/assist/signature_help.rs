use destack_query as query;
use destack_query::SignatureHelp;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a signature_help test.
///
/// Verifies that signature help at cursor position shows expected function signature.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check signature expectations from markers
    for (cursor_idx, (expected_sig, expected_active)) in &session.markers.expectations.signature {
        let Some(cursor) = session.markers.cursor(*cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };

        let result = query::signature_help(&session.session, session.file_id, cursor.offset);

        match result {
            Some(signature_help) => {
                // validate signature help invariants before substring checks
                if let Err(message) = validate_signature_help(&signature_help) {
                    return TestResult::Failed { message };
                }

                let signature = &signature_help.signatures[signature_help.active_signature];
                if !signature.label.contains(expected_sig) {
                    return TestResult::Failed {
                        message: format!(
                            "signature_help at ${} expected '{}', got '{}'",
                            cursor_idx, expected_sig, signature.label
                        ),
                    };
                }

                if let Some(active) = expected_active
                    && signature_help.active_parameter != *active
                {
                    return TestResult::Failed {
                        message: format!(
                            "signature_help at ${cursor_idx} expected active parameter {active}, got {}",
                            signature_help.active_parameter
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("signature_help at ${cursor_idx} returned None"),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve the target position, allowing file scoped targets
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return TestResult::Failed { message },
    };

    // run the signature help query once
    let result = query::signature_help(&session.session, file_id, offset);

    let expected_sig = exp.content.trim();

    // empty expectation is an error
    if expected_sig.is_empty() {
        return TestResult::Failed {
            message: format!(
                "signature_help expectation is empty at '{}', got: {:?}",
                exp.target,
                result.map(|r| r.signatures.first().map(|s| s.label.clone()))
            ),
        };
    }

    // "<none>" means we expect no result
    if expected_sig == "<none>" {
        return match result {
            None => TestResult::Passed,
            Some(sig_help) => TestResult::Failed {
                message: format!(
                    "signature_help at '{}' expected None, got '{}'",
                    exp.target,
                    sig_help
                        .signatures
                        .first()
                        .map(|s| s.label.as_str())
                        .unwrap_or("")
                ),
            },
        };
    }

    match result {
        Some(sig_help) => {
            // validate signature help invariants before comparisons
            if let Err(message) = validate_signature_help(&sig_help) {
                return TestResult::Failed { message };
            }

            // compare against protocol shaped snapshots when structured
            if is_snapshot_expectation(expected_sig) {
                let actual_snapshot = format_signature_help_snapshot(&sig_help);
                let expected_snapshot = normalize_expected_snapshot(expected_sig);

                if actual_snapshot != expected_snapshot {
                    return TestResult::Failed {
                        message: format!(
                            "signature_help snapshot mismatch at '{}'\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}",
                            exp.target
                        ),
                    };
                }

                return TestResult::Passed;
            }

            // fall back to substring matching for legacy expectations
            let sig = &sig_help.signatures[sig_help.active_signature];
            if sig.label.contains(expected_sig) {
                TestResult::Passed
            } else {
                TestResult::Failed {
                    message: format!(
                        "signature_help at '{}' expected '{expected_sig}', got '{}'",
                        exp.target, sig.label
                    ),
                }
            }
        }
        None => TestResult::Failed {
            message: format!("signature_help at '{}' returned None", exp.target),
        },
    }
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains("active_signature=") || line.starts_with("signature["))
}

/// Validate signature help invariants.
fn validate_signature_help(signature_help: &SignatureHelp) -> Result<(), String> {
    let mut errors = Vec::new();

    // require at least one signature
    if signature_help.signatures.is_empty() {
        errors.push("signature_help returned empty signatures".to_string());
    }

    // validate active signature index when signatures exist
    if !signature_help.signatures.is_empty()
        && signature_help.active_signature >= signature_help.signatures.len()
    {
        errors.push(format!(
            "active_signature {} is out of bounds for {} signatures",
            signature_help.active_signature,
            signature_help.signatures.len(),
        ));
    }

    // validate the active parameter against the active signature
    if let Some(active_signature) = signature_help
        .signatures
        .get(signature_help.active_signature)
    {
        if active_signature.label.trim().is_empty() {
            errors.push("active signature label is empty".to_string());
        }

        if active_signature.parameters.is_empty() {
            errors.push("active signature has no parameters".to_string());
        } else if signature_help.active_parameter >= active_signature.parameters.len() {
            errors.push(format!(
                "active_parameter {} is out of bounds for {} parameters",
                signature_help.active_parameter,
                active_signature.parameters.len(),
            ));
        }
    }

    // validate that each signature and parameter has a label
    for (sig_idx, signature) in signature_help.signatures.iter().enumerate() {
        if signature.label.trim().is_empty() {
            errors.push(format!("signature[{sig_idx}] label is empty"));
        }

        for (param_idx, parameter) in signature.parameters.iter().enumerate() {
            if parameter.label.trim().is_empty() {
                errors.push(format!(
                    "signature[{sig_idx}] parameter[{param_idx}] label is empty",
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "signature_help invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Format signature help as a protocol shaped snapshot.
fn format_signature_help_snapshot(signature_help: &SignatureHelp) -> String {
    let mut lines = Vec::new();

    // include the active indices first
    lines.push(format!(
        "active_signature={} active_parameter={}",
        signature_help.active_signature, signature_help.active_parameter,
    ));

    // include each signature with parameter labels and docs
    for (sig_idx, signature) in signature_help.signatures.iter().enumerate() {
        let label = flatten_text(&signature.label);
        let documentation = signature
            .documentation
            .as_deref()
            .map(flatten_text)
            .unwrap_or_else(|| "<none>".to_string());

        let mut parameter_labels = Vec::new();
        let mut parameter_docs = Vec::new();
        for parameter in &signature.parameters {
            parameter_labels.push(flatten_text(&parameter.label));
            let doc = parameter
                .documentation
                .as_deref()
                .map(flatten_text)
                .unwrap_or_else(|| "<none>".to_string());
            parameter_docs.push(doc);
        }

        lines.push(format!(
            "signature[{sig_idx}] label={label} documentation={documentation} parameters={} param_docs={}",
            parameter_labels.join("|"),
            parameter_docs.join("|"),
        ));
    }

    normalize_expected_snapshot(&lines.join("\n"))
}

/// Flatten multi-line text into a single line for snapshots.
fn flatten_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
