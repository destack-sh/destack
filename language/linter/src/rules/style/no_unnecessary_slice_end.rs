use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow slice end arguments that cannot shorten the result.
    pub NO_UNNECESSARY_SLICE_END {
        id: "no-unnecessary-slice-end",
        summary: "Disallow slice end arguments that cannot shorten the result",
        explanation: r#"
Passing the source length or positive infinity as a slice end selects every remaining value.
Instead, you SHOULD omit an end argument that cannot shorten the result.
"#,
        example: {
            reported: r#"
function tail(values: int32[]): int32[] {
    return values.slice(1, values.length as isize);
}
"#,
            accepted: r#"
function tail(values: int32[]): int32[] {
    return values.slice(1);
}
"#,
        },
        provenance: [Unicorn("no-unnecessary-slice-end")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical slice calls bounded by their own source length.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array and string slice calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let Some(member) = module.language_member(expression)? else {
            continue;
        };
        if member != dir::LanguageItem::Array.member("slice")
            && member != dir::LanguageItem::String.member("slice")
        {
            continue;
        }

        // require a start and end argument
        let [start_argument, end_argument] = call.arguments else {
            continue;
        };
        let (dir::Argument::Positional { value: start }, dir::Argument::Positional { value: end }) =
            (view.get(*start_argument), view.get(*end_argument))
        else {
            continue;
        };

        // recognize positive infinity
        let message = if module
            .infinity(*end)?
            .is_some_and(|value| value.is_sign_positive())
        {
            "slice end is positive infinity"
        }
        // otherwise compare the end with its own receiver length
        else {
            let length = match view.get(*end) {
                dir::Expression::As { expression, .. } => *expression,
                _ => *end,
            };
            let dir::Expression::Member {
                left: length_receiver,
                ..
            } = view.get(length)
            else {
                continue;
            };
            if module.language_member(length)? != Some(member.owner.member("length"))
                || !module.is_same_computation(call.receiver, *length_receiver)?
            {
                continue;
            }

            "slice end repeats the source length"
        };

        // retain the start and omit the redundant suffix
        let span = module.source_extent(end.into_any())?;
        let mut diagnostic = lint.diagnostic(message, span);
        if let Some(suggestion) = suggest_open_end(module, lint, expression, *start)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the equivalent open-ended slice call.
fn suggest_open_end(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    start: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // preserve comments outside the retained start expression
    let arguments = module.source_region(expression.into_any(), NodeSpanRegion::Arguments)?;
    let start = module.source_extent(start.into_any())?;
    if module.has_unretained_comment(arguments, &[start])? {
        return Ok(None);
    }

    // retain the exact authored start expression
    let source = module.source(start)?;
    let patch = Patch::replace(arguments, format!("({source})"));
    let suggestion = lint.fix("omit the redundant slice end", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a bound taken from another array.
    #[test]
    fn test_accepts_foreign_length() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_SLICE_END,
            r#"
function prefix(values: int32[], other: int32[]): int32[] {
    return values.slice(0, other.length as isize);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user method with the same name.
    #[test]
    fn test_accepts_user_slice_method() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_SLICE_END,
            r#"
class Values {
    length: int32 = 0;
    slice(start: int32, end: int32): this {
        return this;
    }
}

function copy(values: Values): Values {
    return values.slice(0, values.length);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a string slice bounded by the same string's length.
    #[test]
    fn test_replaces_string_length() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_SLICE_END,
            r#"
function tail(value: string): string {
    return value.slice(1, value.length);
}
"#,
        );

        session.assert_fixes(
            r#"
function tail(value: string): string {
    return value.slice(1);
}
"#,
        );
    }

    /// Replace a positive-infinity string slice end.
    #[test]
    fn test_replaces_positive_infinity() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_SLICE_END,
            r#"
function tail(value: string): string {
    return value.slice(1, Number.POSITIVE_INFINITY);
}
"#,
        );

        session.assert_fixes(
            r#"
function tail(value: string): string {
    return value.slice(1);
}
"#,
        );
    }

    /// Preserve a comment beside the redundant end by omitting the fix.
    #[test]
    fn test_reports_commented_end_without_fix() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_SLICE_END,
            r#"
function tail(value: string): string {
    return value.slice(1, /* retain */ value.length);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unnecessary-slice-end]: slice end repeats the source length
 ──▶ main.tspp:2:40
  │
1 │ function tail(value: string): string {
2 │     return value.slice(1, /* retain */ value.length);
  │                                        ^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
