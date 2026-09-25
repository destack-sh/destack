use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Result projection methods over equivalent pattern matching.
    pub MANUAL_OK_ERR {
        id: "manual-ok-err",
        summary: "Prefer Result projection methods over equivalent pattern matching",
        explanation: r#"
Branching on a Result to select one payload or undefined manually projects the Result.
Instead, you SHOULD use `ok` or `err` to project the selected payload.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): int32 | undefined {
    return match (result) {
        Ok { value } => value
        _ => undefined
    };
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): int32 | undefined {
    return result.ok();
}
"#,
        },
        provenance: [Clippy("manual_ok_err")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Result matches that project one payload as an optional value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect match and if-let Result projections
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let projection = match node {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    continue;
                };

                // reverse only disjoint Result variants, never a leading wildcard
                let projected = projected_method(module, *first, *second)?;
                let projected = if projected.is_some()
                    || matches!(view.get(view.get(*first).pattern()), dir::Pattern::Wildcard)
                {
                    projected
                } else {
                    projected_method(module, *second, *first)?
                };

                projected.map(|method| (*value, method))
            }
            dir::Expression::If { .. } => projected_conditional(module, expression)?,
            _ => None,
        };
        let Some((value, method)) = projection else {
            continue;
        };
        if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Result) {
            continue;
        }
        let value_type = module.node_type_id(value.into_any())?;
        if module.dir.borrow_access(value_type)?.is_some() {
            continue;
        }
        let member = dir::LanguageItem::Result.member(method);
        if module.is_within_language_member(expression.into_any(), member)? {
            continue;
        }

        // replace the complete match with its canonical projection
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("match manually projects a Result payload", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, value, method)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the projection selected by one if-let expression.
fn projected_conditional(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, &'static str)>, ProviderError> {
    let view = module.view();

    // select one unannotated binding condition with two value branches
    let dir::Expression::If {
        form: dir::IfForm::If,
        condition,
        then_expression,
        else_expression: Some(else_expression),
    } = view.get(expression)
    else {
        return Ok(None);
    };
    let Some((_, _, declarator)) = condition.as_binding() else {
        return Ok(None);
    };
    let declarator = view.get(declarator);
    if declarator.ty.is_some() {
        return Ok(None);
    }
    let Some(value) = declarator.value else {
        return Ok(None);
    };
    let Some(payload) = module.sole_value_expression(*then_expression) else {
        return Ok(None);
    };
    let Some(fallback) = module.sole_value_expression(*else_expression) else {
        return Ok(None);
    };
    if view.get(fallback).as_scalar() != Some(dir::Literal::Undefined) {
        return Ok(None);
    }

    // require the selected payload to be returned unchanged
    if module
        .selected_pattern_binding(declarator.pattern, payload)?
        .is_none()
    {
        return Ok(None);
    }
    let method = match module.pattern_language_item(declarator.pattern)? {
        Some(dir::LanguageItem::Ok) => "ok",
        Some(dir::LanguageItem::Err) => "err",
        _ => return Ok(None),
    };

    Ok(Some((value, method)))
}

/// Return the projection method for one payload and fallback arm ordering.
fn projected_method(
    module: &DirModule<'_>,
    payload_arm: dir::LocalNodeId<dir::MatchArm>,
    fallback_arm: dir::LocalNodeId<dir::MatchArm>,
) -> Result<Option<&'static str>, ProviderError> {
    let view = module.view();

    // require unguarded value bodies
    let Some((payload_pattern, payload_body)) = module.match_arm_value(payload_arm) else {
        return Ok(None);
    };
    let Some((fallback_pattern, fallback_body)) = module.match_arm_value(fallback_arm) else {
        return Ok(None);
    };
    if view.get(fallback_body).as_scalar() != Some(dir::Literal::Undefined) {
        return Ok(None);
    }

    // require the selected Result payload to be returned unchanged
    if module
        .selected_pattern_binding(payload_pattern, payload_body)?
        .is_none()
    {
        return Ok(None);
    }
    let (method, opposite) = match module.pattern_language_item(payload_pattern)? {
        Some(dir::LanguageItem::Ok) => ("ok", dir::LanguageItem::Err),
        Some(dir::LanguageItem::Err) => ("err", dir::LanguageItem::Ok),
        _ => return Ok(None),
    };

    // require a wildcard or the opposite Result variant as the fallback
    if !matches!(view.get(fallback_pattern), dir::Pattern::Wildcard)
        && module.pattern_language_item(fallback_pattern)? != Some(opposite)
    {
        return Ok(None);
    }

    Ok(Some(method))
}

/// Build one Result payload projection.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
    method: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping around the Result expression
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{value}.{method}()"));
    let suggestion = lint.fix("project the Result payload directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace error projection with reversed arm ordering.
    #[test]
    fn test_replaces_err_projection() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function error(result: Result<int32, string>): string | undefined {
    return match (result) {
        Ok { value: _ } => undefined
        Err { error } => error
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function error(result: Result<int32, string>): string | undefined {
    return result.err();
}
"#,
        );
    }

    /// Replace projection with an explicit opposite Result variant.
    #[test]
    fn test_replaces_explicit_fallback() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function value(result: Result<int32, string>): int32 | undefined {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => undefined
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function value(result: Result<int32, string>): int32 | undefined {
    return result.ok();
}
"#,
        );
    }

    /// Replace an if-let Result projection.
    #[test]
    fn test_replaces_conditional_projection() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function value(result: Result<int32, string>): int32 | undefined {
    return if (let Ok { value } = result) {
        value
    } else {
        undefined
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function value(result: Result<int32, string>): int32 | undefined {
    return result.ok();
}
"#,
        );
    }

    /// Accept a transformed Result payload.
    #[test]
    fn test_accepts_transformed_payload() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function increment(result: Result<int32, string>): int32 | undefined {
    return match (result) {
        Ok { value } => value + 1
        _ => undefined
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a fallback that preserves information.
    #[test]
    fn test_accepts_nonundefined_fallback() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        _ => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept payload projection from a raw variant union.
    #[test]
    fn test_accepts_raw_variant_union() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function value(result: Ok<int32> | Err<string>): int32 | undefined {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => undefined
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve projection from a borrowed Result that cannot call a consuming method.
    #[test]
    fn test_accepts_borrowed_result() {
        let session = TestSession::dir(
            &MANUAL_OK_ERR,
            r#"
function value<'a>(result: &'a readonly Result<int32, string>): int32 | undefined {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => undefined
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
