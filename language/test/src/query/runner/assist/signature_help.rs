use destack_query as query;
use destack_query::SignatureHelp;

use crate::core::CaseResult;
use crate::query::runner::expectation::{
    TextExpectation, describe_text_expectation, matches_text_expectation,
};
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{
    compare_snapshot, looks_like_snapshot, normalize_expected_snapshot,
};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a signature_help test.
///
/// Verifies that signature help at cursor position shows expected function signature.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no signature_help expectation provided".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the target position, allowing file scoped targets
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // run the signature help query once
    let ctx = session.module_context(file_id);
    let result = query::signature_help(&ctx, offset);

    let expected_sig = exp.content.trim();

    // empty expectation is an error
    if expected_sig.is_empty() {
        return CaseResult::Failed {
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
            None => CaseResult::Passed,
            Some(sig_help) => CaseResult::Failed {
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
                return CaseResult::Failed { message };
            }

            // compare against protocol shaped snapshots when structured
            if looks_like_snapshot(expected_sig, &["active_signature=", "signature["]) {
                let actual_snapshot = format_signature_help_snapshot(&sig_help);
                return compare_snapshot(
                    &format!("signature_help at '{}'", exp.target),
                    &actual_snapshot,
                    expected_sig,
                );
            }

            // explicit query blocks are exact only
            let sig = &sig_help.signatures[sig_help.active_signature];
            let expectation = match TextExpectation::parse(expected_sig) {
                Ok(expectation) => expectation,
                Err(message) => return CaseResult::Failed { message },
            };
            if matches_text_expectation(&sig.label, expectation) {
                CaseResult::Passed
            } else {
                CaseResult::Failed {
                    message: format!(
                        "signature_help at '{}' expected to {} '{}', got '{}'",
                        exp.target,
                        describe_text_expectation(expectation),
                        expectation.text(),
                        sig.label
                    ),
                }
            }
        }
        None => CaseResult::Failed {
            message: format!("signature_help at '{}' returned None", exp.target),
        },
    }
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
