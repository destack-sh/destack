use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, NodeSpanRegion};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer first and last accessors over equivalent indexing.
    pub PREFER_FIRST_LAST {
        id: "prefer-first-last",
        summary: "Prefer first and last accessors over equivalent indexing",
        explanation: r#"
`array.at(0)` and `array.at(-1)` duplicate the optional `first` and `last` endpoint lookups.
`array.at(array.length - 1)` also duplicates `last`.
Instead, you SHOULD use the corresponding endpoint accessor.

Trapping subscript access has different empty-array behavior and remains unchanged.
"#,
        example: {
            reported: r#"
function first(values: int32[]): int32 | undefined {
    return values.at(0);
}
"#,
            accepted: r#"
function first(values: int32[]): int32 | undefined {
    return values.first();
}
"#,
        },
        provenance: [Clippy("get_first"), Clippy("get_last_with_len")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical endpoint lookups written as `at` calls.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical array at calls with one endpoint index
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("at")) {
            continue;
        }

        // reserve filtered endpoints for prefer-array-search
        if module.language_member(call.receiver)? == Some(dir::LanguageItem::Array.member("filter"))
        {
            continue;
        }

        // read the array receiver and sole index value
        let receiver = call.receiver;
        let [argument] = call.arguments else {
            continue;
        };
        let Some(index) = view.get(*argument).value() else {
            continue;
        };

        // recognize literal endpoint indices directly
        let name = match module.scalar_constant(index)? {
            Some(dir::Literal::Integer(0)) => "first",
            Some(dir::Literal::Integer(-1)) => "last",
            _ => {
                // recognize the canonical array length minus one
                let dir::Expression::Binary {
                    left: length,
                    operator: dir::BinaryOperator::Subtract,
                    right: offset,
                } = view.get(index)
                else {
                    continue;
                };
                let dir::Expression::Member {
                    left: length_receiver,
                    ..
                } = view.get(*length)
                else {
                    continue;
                };
                if module.language_member(*length)?
                    != Some(dir::LanguageItem::Array.member("length"))
                    || module.scalar_constant(*offset)? != Some(dir::Literal::Integer(1))
                    || !module.is_same_computation(receiver, *length_receiver)?
                {
                    continue;
                }

                "last"
            }
        };

        // replace only the selected member name and argument list
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("endpoint lookup uses a numeric index", span);
        if let Some(suggestion) = suggestion(module, lint, expression, call.callee, name)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one endpoint accessor call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    callee: dir::LocalNodeId<dir::Expression>,
    name: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let member_name = module.main_span(callee.into_any())?;
    let arguments = module.source_region(expression.into_any(), NodeSpanRegion::Arguments)?;
    if module.has_unretained_comment(arguments, &[])? {
        return Ok(None);
    }

    // retain the receiver and optional-chain operators
    let mut file = FilePatch::new(extent.file);
    file.replace(member_name, name);
    file.replace(arguments, "()");
    file.sort();
    let suggestion = lint.fix("use the endpoint accessor", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace negative-one lookup with `last`.
    #[test]
    fn test_replaces_negative_one_with_last() {
        let session = TestSession::dir(
            &PREFER_FIRST_LAST,
            r#"
function last(values: int32[]): int32 | undefined {
    return values.at(-1);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-first-last]: endpoint lookup uses a numeric index
 ──▶ main.tspp:2:12
  │
1 │ function last(values: int32[]): int32 | undefined {
2 │     return values.at(-1);
  │            ^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the endpoint accessor
--- a/main.tspp
+++ b/main.tspp

    1│ function last(values: int32[]): int32 | undefined {
-   2│     return values.at(-1);
+   2│     return values.last();
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function last(values: int32[]): int32 | undefined {
    return values.last();
}
"#,
        );
    }

    /// Replace a length-minus-one lookup with `last`.
    #[test]
    fn test_replaces_length_minus_one_with_last() {
        let session = TestSession::dir(
            &PREFER_FIRST_LAST,
            r#"
function last(values: int32[]): int32 | undefined {
    return values.at(values.length - 1);
}
"#,
        );

        session.assert_fixes(
            r#"
function last(values: int32[]): int32 | undefined {
    return values.last();
}
"#,
        );
    }

    /// Preserve an optional receiver when replacing its endpoint lookup.
    #[test]
    fn test_replaces_optional_endpoint() {
        let session = TestSession::dir(
            &PREFER_FIRST_LAST,
            r#"
function first(values: int32[] | undefined): int32 | undefined {
    return values?.at(0);
}
"#,
        );

        session.assert_fixes(
            r#"
function first(values: int32[] | undefined): int32 | undefined {
    return values?.first();
}
"#,
        );
    }

    /// Accept trapping subscript access at index zero.
    #[test]
    fn test_accepts_trapping_first_subscript() {
        let session = TestSession::dir(
            &PREFER_FIRST_LAST,
            r#"
function first(values: int32[]): int32 {
    return values[0];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Leave filtered endpoints to the direct Array search rule.
    #[test]
    fn test_accepts_filtered_endpoint() {
        let session = TestSession::dir(
            &PREFER_FIRST_LAST,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments inside the index list by omitting the fix.
    #[test]
    fn test_reports_commented_endpoint_without_fix() {
        let session = TestSession::dir(
            &PREFER_FIRST_LAST,
            r#"
function first(values: int32[]): int32 | undefined {
    return values.at(/* retain */ 0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-first-last]: endpoint lookup uses a numeric index
 ──▶ main.tspp:2:12
  │
1 │ function first(values: int32[]): int32 | undefined {
2 │     return values.at(/* retain */ 0);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
