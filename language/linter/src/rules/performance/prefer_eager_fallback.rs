use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer eager fallbacks when deferred evaluation cannot avoid work.
    pub PREFER_EAGER_FALLBACK {
        id: "prefer-eager-fallback",
        summary: "Prefer eager fallbacks when deferred evaluation cannot avoid work",
        explanation: r#"
`Result.unwrapOrElse` adds an unnecessary callback when its fallback is `Copy` and effect-free.
Instead, you SHOULD pass the expression directly to `unwrapOr`.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => 0);
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        },
        provenance: [Clippy("unnecessary_lazy_evaluations")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Result fallback callbacks whose values can be evaluated eagerly.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect canonical Result.unwrapOrElse calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some((callee, callback, value)) =
            select_eager_fallback(module, expression, &occurrences)?
        else {
            continue;
        };

        // replace the callback with its returned expression
        let span = module.source_extent(callback.into_any())?;
        let mut diagnostic = lint.diagnostic("fallback callback only returns an eager value", span);
        if let Some(fix) = build_fix(module, lint, expression, callee, callback, value)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the eager value returned by one Result.unwrapOrElse callback.
fn select_eager_fallback(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<
    Option<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || module.language_member(expression)?
            != Some(dir::LanguageItem::Result.member("unwrapOrElse"))
    {
        return Ok(None);
    }
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: callback } = module.view().get(*argument) else {
        return Ok(None);
    };

    // require a synchronous callback with zero or one unused direct parameter
    let Some(lambda) = module.lambda(*callback) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    match lambda.signature.parameters.as_slice() {
        [] => {}
        [parameter]
            if matches!(
                module.view().get(*parameter),
                dir::Parameter::Named {
                    default: None,
                    is_optional: false,
                    ..
                }
            ) && module
                .binding_uses_within(
                    module.declaration_symbol(*parameter)?,
                    lambda
                        .body
                        .map_or(callback.into_any(), |body| body.into_any()),
                    occurrences,
                )
                .is_empty() => {}
        _ => return Ok(None),
    }
    let Some(value) = lambda
        .body
        .and_then(|body| module.sole_value_expression(body))
    else {
        return Ok(None);
    };
    let value_type = module.adjusted_type_id(value.into_any())?;
    if !module.is_speculatable_expression(value)? || !module.satisfies_copy(value_type)? {
        return Ok(None);
    }

    Ok(Some((call.callee, *callback, value)))
}

/// Replace one eager fallback callback with its returned expression.
fn build_fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    callee: dir::LocalNodeId<dir::Expression>,
    callback: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let callback = module.source_extent(callback.into_any())?;
    let value = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(callback, &[value])? {
        return Ok(None);
    }

    // retain the callback value and select the eager method
    let value = module.source(value)?;
    let mut patch = FilePatch::new(extent.file);
    patch.replace(module.main_span(callee.into_any())?, "unwrapOr");
    patch.replace(callback, value);
    patch.sort();
    let fix = lint.fix("pass the fallback value directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Inline a trivial Result fallback callback.
    #[test]
    fn test_inlines_trivial_fallback() {
        let session = TestSession::dir(
            &PREFER_EAGER_FALLBACK,
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => 0);
}
"#,
        );

        session.assert_fixes(
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        );
    }

    /// Inline a fallback callback whose unused error parameter is explicit.
    #[test]
    fn test_inlines_error_independent_fallback() {
        let session = TestSession::dir(
            &PREFER_EAGER_FALLBACK,
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse((_error) => 0);
}
"#,
        );

        session.assert_fixes(
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        );
    }

    /// Accept a callback that uses the Result error.
    #[test]
    fn test_accepts_error_dependent_fallback() {
        let session = TestSession::dir(
            &PREFER_EAGER_FALLBACK,
            r#"
function value(result: Result<isize, string>): isize {
    return result.unwrapOrElse((error) => error.length);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an effectful deferred fallback.
    #[test]
    fn test_accepts_effectful_fallback() {
        let session = TestSession::dir(
            &PREFER_EAGER_FALLBACK,
            r#"
declare function recover(): int32;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => recover());
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
