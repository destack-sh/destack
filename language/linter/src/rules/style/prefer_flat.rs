use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Array.flat over spreading nested arrays into concat.
    pub PREFER_FLAT {
        id: "prefer-flat",
        summary: "Prefer Array.flat over spreading nested arrays into concat",
        explanation: r#"
Spreading nested arrays into `concat` manually performs one-level flattening.
Instead, you SHOULD call `flat` on the nested array.
"#,
        example: {
            reported: r#"
function flatten(values: int32[][]): int32[] {
    return Array<int32>.new().concat(...values);
}
"#,
            accepted: r#"
function flatten(values: int32[][]): int32[] {
    return values.flat();
}
"#,
        },
        provenance: [Unicorn("prefer-array-flat")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report one-level flattening expressed through an empty array and concat.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical Array.concat calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || !call.generic_arguments.is_empty()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Array.member("concat"))
            || !is_empty_array(module, call.receiver)?
        {
            continue;
        }

        // require one spread of a canonical nested array
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Spread { value } = module.view().get(*argument) else {
            continue;
        };
        if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Array)
            || !module.is_speculatable_expression(*value)?
        {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("concat manually flattens a nested array", span);
        if let Some(suggestion) = suggestion(module, lint, span, *value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one concat receiver constructs an empty canonical array.
fn is_empty_array(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    if matches!(
        module.view().get(expression),
        dir::Expression::ArrayExpression { elements } if elements.is_empty()
    ) {
        return Ok(true);
    }

    // recognize the canonical zero argument array factory
    let Some(call) = module.member_call(expression) else {
        return Ok(false);
    };
    let is_empty = !call.is_optional()
        && call.generic_arguments.is_empty()
        && call.arguments.is_empty()
        && module.language_member(expression)? == Some(dir::LanguageItem::Array.member("new"));

    Ok(is_empty)
}

/// Replace manual one-level flattening with Array.flat.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    array: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let array_extent = module.source_extent(array.into_any())?;
    if module.has_unretained_comment(extent, &[array_extent])? {
        return Ok(None);
    }

    let array = module.expression_source(array, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{array}.flat()"));
    let suggestion = lint.suggestion("call flat directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an empty array concat with flat.
    #[test]
    fn test_replaces_spread_concat() {
        let session = TestSession::dir(
            &PREFER_FLAT,
            r#"
function flatten(values: int32[][]): int32[] {
    return Array<int32>.new().concat(...values);
}
"#,
        );

        session.assert_suggestions(
            r#"
function flatten(values: int32[][]): int32[] {
    return values.flat();
}
"#,
        );
    }

    /// Accept concat that retains a leading element.
    #[test]
    fn test_accepts_nonempty_receiver() {
        let session = TestSession::dir(
            &PREFER_FLAT,
            r#"
function flatten(values: int32[][]): int32[] {
    return [0].concat(...values);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined concat method.
    #[test]
    fn test_accepts_user_method() {
        let session = TestSession::dir(
            &PREFER_FLAT,
            r#"
class Values {
    concat(...values: int32[][]): int32[] {
        return [];
    }
}

function flatten(target: Values, values: int32[][]): int32[] {
    return target.concat(...values);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a nested array expression whose evaluation may perform work.
    #[test]
    fn test_accepts_effectful_array() {
        let session = TestSession::dir(
            &PREFER_FLAT,
            r#"
declare function values(): int32[][];

function flatten(): int32[] {
    return Array<int32>.new().concat(...values());
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
