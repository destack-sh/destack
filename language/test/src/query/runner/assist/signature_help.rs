use destack_workspace::query;

use crate::harness::TestResult;
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
            Some(sig_help) => {
                if sig_help.signatures.is_empty() {
                    return TestResult::Failed {
                        message: format!(
                            "signature_help at ${cursor_idx} returned empty signatures"
                        ),
                    };
                }

                let sig = &sig_help.signatures[sig_help.active_signature];
                if !sig.label.contains(expected_sig) {
                    return TestResult::Failed {
                        message: format!(
                            "signature_help at ${} expected '{}', got '{}'",
                            cursor_idx, expected_sig, sig.label
                        ),
                    };
                }

                if let Some(active) = expected_active
                    && sig_help.active_parameter != *active
                {
                    return TestResult::Failed {
                        message: format!(
                            "signature_help at ${cursor_idx} expected active parameter {active}, got {}",
                            sig_help.active_parameter
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
    // parse cursor from target
    let cursor_idx: usize = exp
        .target
        .strip_prefix('$')
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let Some(cursor) = session.markers.cursor(cursor_idx) else {
        return TestResult::Failed {
            message: format!("cursor ${cursor_idx} not found"),
        };
    };

    let result = query::signature_help(&session.session, session.file_id, cursor.offset);

    let expected_sig = exp.content.trim();

    // empty expectation is an error
    if expected_sig.is_empty() {
        return TestResult::Failed {
            message: format!(
                "signature_help expectation is empty at ${cursor_idx}, got: {:?}",
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
                    "signature_help at ${cursor_idx} expected None, got '{}'",
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
            if sig_help.signatures.is_empty() {
                return TestResult::Failed {
                    message: format!("signature_help at ${cursor_idx} returned empty signatures"),
                };
            }

            let sig = &sig_help.signatures[sig_help.active_signature];
            if !sig.label.contains(expected_sig) {
                TestResult::Failed {
                    message: format!(
                        "signature_help at ${cursor_idx} expected '{expected_sig}', got '{}'",
                        sig.label
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("signature_help at ${cursor_idx} returned None"),
        },
    }
}
