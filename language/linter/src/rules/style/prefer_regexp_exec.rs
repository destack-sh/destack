use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `RegExp.exec` when regular-expression match details are consumed.
    pub PREFER_REGEXP_EXEC {
        id: "prefer-regexp-exec",
        summary: "Prefer `RegExp.exec` when regular-expression match details are consumed",
        explanation: r#"
`String.match` changes its result shape for global patterns and places matching behavior on the input string.
Instead, you SHOULD call `RegExp.exec` for a non-global pattern when consuming one match record.

Global patterns remain with `String.match` because they collect every match rather than one match record.
"#,
        example: {
            reported: r#"
function firstMatch(text: string): unknown {
    return text.match(/[a-z]+/i);
}
"#,
            accepted: r#"
function firstMatch(text: string): unknown {
    return /[a-z]+/i.exec(text);
}
"#,
        },
        provenance: [TypeScriptEslint("prefer-regexp-exec")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report non-global regular-expression literals passed to String.match.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical String.match calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::String.member("match"))
        {
            continue;
        }

        // retain boolean-only consumption because it ignores the match record
        if is_only_tested(module, expression)? {
            continue;
        }

        // require one direct non-global regular-expression literal
        let [pattern] = call.arguments else {
            continue;
        };
        let Some(pattern) = view.get(*pattern).value() else {
            continue;
        };
        let dir::Expression::Literal(dir::Literal::RegexString { flags, .. }) = view.get(pattern)
        else {
            continue;
        };
        let is_global = flags.is_some_and(|flags| module.dir.strings.get(flags).contains('g'));
        if is_global {
            continue;
        }

        // replace the complete call with pattern-first execution
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("String.match consumes one match record", span);
        if let Some(fix) = fix(module, lint, expression, pattern, call.receiver)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one match call is used only in a null comparison.
fn is_only_tested(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let view = module.view();
    let Some(parent) = view.get_parent_for(expression) else {
        return Ok(false);
    };
    let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
        return Ok(false);
    };
    let dir::Expression::Binary {
        left,
        operator,
        right,
    } = view.get(parent)
    else {
        return Ok(false);
    };
    if !operator.is_strict_equality() {
        return Ok(false);
    }

    let is_test = [(*left, *right), (*right, *left)]
        .into_iter()
        .any(|(value, absence)| {
            value == expression && view.get(absence).as_scalar() == Some(dir::Literal::Null)
        });

    Ok(is_test)
}

/// Build one pattern-first regular-expression execution.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    pattern: dir::LocalNodeId<dir::Expression>,
    input: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let pattern_span = module.source_extent(pattern.into_any())?;
    let input_span = module.source_extent(input.into_any())?;
    if module.has_unretained_comment(extent, &[pattern_span, input_span])? {
        return Ok(None);
    }

    // retain exact sources and group the postfix pattern when required
    let pattern = module.expression_source(pattern, dir::OperatorPrecedence::Postfix)?;
    let input = module.source(input_span)?;
    let replacement = format!("{pattern}.exec({input})");
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("execute the regular expression directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a consumed non-global match with direct regular-expression execution.
    #[test]
    fn test_replaces_non_global_match() {
        let session = TestSession::dir(
            &PREFER_REGEXP_EXEC,
            r#"
function firstMatch(text: string): unknown {
    return text.match(/[a-z]+/i);
}
"#,
        );

        session.assert_fixes(
            r#"
function firstMatch(text: string): unknown {
    return /[a-z]+/i.exec(text);
}
"#,
        );
    }

    /// Accept a global pattern that collects all matches.
    #[test]
    fn test_accepts_global_match() {
        let session = TestSession::dir(
            &PREFER_REGEXP_EXEC,
            r#"
function matches(text: string): unknown {
    return text.match(/[a-z]+/g);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept boolean-only match consumption.
    #[test]
    fn test_accepts_match_test() {
        let session = TestSession::dir(
            &PREFER_REGEXP_EXEC,
            r#"
function contains(text: string): boolean {
    return text.match(/[a-z]+/) !== null;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
