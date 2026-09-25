use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer unwrapOrDefault over equivalent Result pattern matching.
    pub MANUAL_UNWRAP_OR_DEFAULT {
        id: "manual-unwrap-or-default",
        summary: "Prefer unwrapOrDefault over equivalent Result pattern matching",
        explanation: r#"
Branching on a Result to select its successful payload or the payload type's default manually unwraps the Result.
Instead, you SHOULD use `unwrapOrDefault` to state the defaulting operation directly.
"#,
        example: {
            reported: r#"
function value<T: Default>(result: Result<T, string>): T {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => T.default()
    };
}
"#,
            accepted: r#"
function value<T: Default>(result: Result<T, string>): T {
    return result.unwrapOrDefault();
}
"#,
        },
        provenance: [Clippy("manual_unwrap_or_default")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Result matches that manually return the success type's default.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect match and if-let Result defaults
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let default = match node {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    continue;
                };

                // reverse only disjoint Result variants, never a leading wildcard
                let is_default = is_default_match(module, *first, *second)?;
                let is_default = if is_default
                    || matches!(view.get(view.get(*first).pattern()), dir::Pattern::Wildcard)
                {
                    is_default
                } else {
                    is_default_match(module, *second, *first)?
                };

                is_default.then_some(*value)
            }
            dir::Expression::If { .. } => default_conditional(module, expression)?,
            _ => None,
        };
        let Some(value) = default else {
            continue;
        };
        if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Result) {
            continue;
        }
        let value_type = module.node_type_id(value.into_any())?;
        if module.dir.borrow_access(value_type)?.is_some() {
            continue;
        }
        let member = dir::LanguageItem::Result.member("unwrapOrDefault");
        if module.is_within_language_member(expression.into_any(), member)? {
            continue;
        }

        // replace the complete match with unwrapOrDefault
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("match manually unwraps a default Result value", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the Result selected by one if-let default expression.
fn default_conditional(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();

    // select one unannotated successful binding condition with a default
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
    if declarator.ty.is_some()
        || module.pattern_language_item(declarator.pattern)? != Some(dir::LanguageItem::Ok)
    {
        return Ok(None);
    }
    let Some(value) = declarator.value else {
        return Ok(None);
    };
    let Some(payload) = module.sole_value_expression(*then_expression) else {
        return Ok(None);
    };
    let Some(default) = module.sole_value_expression(*else_expression) else {
        return Ok(None);
    };

    // require the successful payload to be returned unchanged
    if module
        .selected_pattern_binding(declarator.pattern, payload)?
        .is_none()
    {
        return Ok(None);
    }

    // require the fallback to call canonical Default.default
    if module.language_member(default)? != Some(dir::LanguageItem::Default.member("default")) {
        return Ok(None);
    }
    let dir::Expression::Call { arguments, .. } = view.get(default) else {
        return Ok(None);
    };
    if !arguments.is_empty() {
        return Ok(None);
    }
    let payload_type = module
        .dir
        .strip_form(module.node_type_id(payload.into_any())?)?;
    let default_type = module
        .dir
        .strip_form(module.node_type_id(default.into_any())?)?;
    if payload_type != default_type {
        return Ok(None);
    }

    Ok(Some(value))
}

/// Return whether two arms return the success payload or its canonical default.
fn is_default_match(
    module: &DirModule<'_>,
    success_arm: dir::LocalNodeId<dir::MatchArm>,
    default_arm: dir::LocalNodeId<dir::MatchArm>,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // require the successful payload to be returned unchanged
    let Some((success_pattern, success_body)) = module.match_arm_value(success_arm) else {
        return Ok(false);
    };
    if module.pattern_language_item(success_pattern)? != Some(dir::LanguageItem::Ok) {
        return Ok(false);
    }
    if module
        .selected_pattern_binding(success_pattern, success_body)?
        .is_none()
    {
        return Ok(false);
    }

    // require an error arm returning canonical Default.default
    let Some((default_pattern, default_body)) = module.match_arm_value(default_arm) else {
        return Ok(false);
    };
    if (!matches!(view.get(default_pattern), dir::Pattern::Wildcard)
        && module.pattern_language_item(default_pattern)? != Some(dir::LanguageItem::Err))
        || module.language_member(default_body)?
            != Some(dir::LanguageItem::Default.member("default"))
    {
        return Ok(false);
    }
    let dir::Expression::Call { arguments, .. } = view.get(default_body) else {
        return Ok(false);
    };
    let payload_type = module
        .dir
        .strip_form(module.node_type_id(success_body.into_any())?)?;
    let default_type = module
        .dir
        .strip_form(module.node_type_id(default_body.into_any())?)?;

    Ok(arguments.is_empty() && payload_type == default_type)
}

/// Build one unwrapOrDefault call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    result: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(result.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping around the Result expression
    let result = module.expression_source(result, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{result}.unwrapOrDefault()"));
    let suggestion = lint.fix("use unwrapOrDefault", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Recognize reversed Result arm ordering.
    #[test]
    fn test_replaces_reversed_arms() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return match (result) {
        Err { error: _ } => T.default()
        Ok { value } => value
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return result.unwrapOrDefault();
}
"#,
        );
    }

    /// Replace an if-let Result default.
    #[test]
    fn test_replaces_conditional_default() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return if (let Ok { value } = result) {
        value
    } else {
        T.default()
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return result.unwrapOrDefault();
}
"#,
        );
    }

    /// Replace an exhaustive wildcard with the canonical default.
    #[test]
    fn test_replaces_wildcard_default() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return match (result) {
        Ok { value } => value
        _ => T.default()
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return result.unwrapOrDefault();
}
"#,
        );
    }

    /// Accept a literal fallback that is not canonical default construction.
    #[test]
    fn test_accepts_literal_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
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

    /// Accept default construction for only one member of the success type.
    #[test]
    fn test_accepts_different_default_type() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
            r#"
function value<T: Default, U: Default>(result: Result<T | U, void>): T | U {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => T.default()
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept default selection from a raw variant union.
    #[test]
    fn test_accepts_raw_variant_union() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
            r#"
function value<T: Default>(result: Ok<T> | Err<string>): T {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => T.default()
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve default selection from a borrowed Result.
    #[test]
    fn test_accepts_borrowed_result() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR_DEFAULT,
            r#"
function value<T: Default & Copy>(result: &readonly Result<T, string>): T {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => T.default()
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
