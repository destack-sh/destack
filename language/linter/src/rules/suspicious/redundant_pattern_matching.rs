use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer canonical result predicates over boolean pattern matches.
    pub REDUNDANT_PATTERN_MATCHING {
        id: "redundant-pattern-matching",
        summary: "Prefer canonical result predicates over boolean pattern matches",
        explanation: r#"
A match that maps `Ok` and `Err` directly to opposite boolean values only tests the result variant.
Instead, you SHOULD call `isOk()` or `isErr()` on the result.
"#,
        example: {
            reported: r#"
function succeeded(result: Result<int32, string>): boolean {
    return match (result) {
        Ok { value: _ } => true
        Err { error: _ } => false
    };
}
"#,
            accepted: r#"
function succeeded(result: Result<int32, string>): boolean {
    return result.isOk();
}
"#,
        },
        provenance: [Clippy("redundant_pattern_matching")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report result matches whose arms only identify the selected variant.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect exhaustive two arm matches
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { value, arms } = node else {
            continue;
        };
        let [first, second] = arms.as_slice() else {
            continue;
        };
        let Some(first) = result_arm(module, *first)? else {
            continue;
        };
        let Some(second) = result_arm(module, *second)? else {
            continue;
        };

        // require opposite variants and opposite boolean results
        if first.variant == second.variant || first.result == second.result {
            continue;
        }
        let predicate = match (first.variant, first.result) {
            (dir::LanguageItem::Ok, true) | (dir::LanguageItem::Err, false) => "isOk",
            (dir::LanguageItem::Ok, false) | (dir::LanguageItem::Err, true) => "isErr",
            _ => continue,
        };

        // replace the complete match when no arm comments are discarded
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("match only tests the result variant", span);
        if let Some(suggestion) = suggestion(module, lint, span, *value, predicate)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One boolean result arm.
#[derive(Debug, Clone, Copy)]
struct ResultArm {
    /// The matched result variant.
    variant: dir::LanguageItem,
    /// The returned boolean value.
    result: bool,
}

/// Return one unguarded result variant arm with a boolean body.
fn result_arm(
    module: &DirModule<'_>,
    arm: dir::LocalNodeId<dir::MatchArm>,
) -> Result<Option<ResultArm>, ProviderError> {
    let view = module.view();
    let dir::MatchArm::Expression {
        pattern,
        guard: None,
        body,
    } = view.get(arm)
    else {
        return Ok(None);
    };
    let Some(result) = view.get(*body).as_boolean() else {
        return Ok(None);
    };
    let Some(variant) = module.pattern_language_item(*pattern)? else {
        return Ok(None);
    };
    if !matches!(variant, dir::LanguageItem::Ok | dir::LanguageItem::Err) {
        return Ok(None);
    }

    Ok(Some(ResultArm { variant, result }))
}

/// Build one canonical result predicate call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
    predicate: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the selected value as the predicate receiver
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{value}.{predicate}()"));
    let suggestion = lint.suggestion("use the result predicate", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace the inverse boolean mapping with `isErr()`.
    #[test]
    fn test_replaces_error_test() {
        let session = TestSession::dir(
            &REDUNDANT_PATTERN_MATCHING,
            r#"
function failed(result: Result<int32, string>): boolean {
    return match (result) {
        Ok { value: _ } => false
        Err { error: _ } => true
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function failed(result: Result<int32, string>): boolean {
    return result.isErr();
}
"#,
        );
    }

    /// Accept a match whose successful arm computes another value.
    #[test]
    fn test_accepts_non_boolean_arm() {
        let session = TestSession::dir(
            &REDUNDANT_PATTERN_MATCHING,
            r#"
function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a boolean match with a guarded arm.
    #[test]
    fn test_accepts_guarded_arm() {
        let session = TestSession::dir(
            &REDUNDANT_PATTERN_MATCHING,
            r#"
function succeeded(result: Result<int32, string>): boolean {
    return match (result) {
        Ok { value } if (value > 0) => true
        Ok { value: _ } => false
        Err { error: _ } => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
