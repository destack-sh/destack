use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer negative indices in equivalent at calls.
    pub PREFER_NEGATIVE_INDEX {
        id: "prefer-negative-index",
        summary: "Prefer negative indices in equivalent at calls",
        explanation: r#"
`array.at(array.length as isize - offset)` performs the same lookup as `array.at(-offset)` for a positive constant offset.
Instead, you SHOULD pass the negative offset directly.
"#,
        example: {
            reported: r#"
function penultimate(values: int32[]): int32 | undefined {
    return values.at((values.length as isize) - 2);
}
"#,
            accepted: r#"
function penultimate(values: int32[]): int32 | undefined {
    return values.at(-2);
}
"#,
        },
        provenance: [Unicorn("prefer-negative-index")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report array `at` calls that manually offset from the same array length.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical array at calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("at")) {
            continue;
        }

        // read the array receiver and sole index value
        let receiver = call.receiver;
        let [argument] = call.arguments else {
            continue;
        };
        let Some(argument) = view.get(*argument).value() else {
            continue;
        };

        // decompose a converted length minus one offset
        let dir::Expression::Binary {
            left: converted_length,
            operator: dir::BinaryOperator::Subtract,
            right: offset,
        } = view.get(argument)
        else {
            continue;
        };
        let dir::Expression::As {
            expression: length, ..
        } = view.get(*converted_length)
        else {
            continue;
        };

        // require the canonical length of the same array
        let dir::Expression::Member {
            left: length_receiver,
            ..
        } = view.get(*length)
        else {
            continue;
        };
        if module.language_member(*length)? != Some(dir::LanguageItem::Array.member("length")) {
            continue;
        }
        if !module.is_same_computation(receiver, *length_receiver)? {
            continue;
        }

        // require a constant offset not handled by prefer-first-last
        let Some(dir::Literal::Integer(value)) = module.scalar_constant(*offset)? else {
            continue;
        };
        if value <= 1 {
            continue;
        }

        // replace the complete manual index with its negative offset
        let span = module.source_extent(argument.into_any())?;
        let mut diagnostic =
            lint.diagnostic("at index is expressed relative to array length", span);
        if let Some(suggestion) = suggestion(module, lint, span, *offset)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the equivalent negative index.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    offset: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let offset_span = module.source_extent(offset.into_any())?;
    if module.has_unretained_comment(extent, &[offset_span])? {
        return Ok(None);
    }

    // negate the exact positive offset
    let offset = module.expression_source(offset, dir::OperatorPrecedence::Prefix)?;
    let replacement = format!("-{offset}");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use a negative index", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a length offset taken from another array.
    #[test]
    fn test_accepts_foreign_length_receiver() {
        let session = TestSession::dir(
            &PREFER_NEGATIVE_INDEX,
            r#"
function read(values: int32[], other: int32[]): int32 | undefined {
    return values.at((other.length as isize) - 2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined method named `at`.
    #[test]
    fn test_accepts_user_at_method() {
        let session = TestSession::dir(
            &PREFER_NEGATIVE_INDEX,
            r#"
class Values {
    length: int32 = 2;
    at(index: int32): int32 | undefined {
        return undefined;
    }
}
function read(values: Values): int32 | undefined {
    return values.at(values.length - 2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept the dedicated last-element offset handled by `last`.
    #[test]
    fn test_accepts_length_minus_one() {
        let session = TestSession::dir(
            &PREFER_NEGATIVE_INDEX,
            r#"
function last(values: int32[]): int32 | undefined {
    return values.at((values.length as isize) - 1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a nonconstant offset from the array length.
    #[test]
    fn test_accepts_dynamic_length_offset() {
        let session = TestSession::dir(
            &PREFER_NEGATIVE_INDEX,
            r#"
function read(values: int32[], offset: isize): int32 | undefined {
    return values.at((values.length as isize) - offset);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a folded positive offset with the equivalent negative index.
    #[test]
    fn test_replaces_constant_length_offset() {
        let session = TestSession::dir(
            &PREFER_NEGATIVE_INDEX,
            r#"
function read(values: int32[]): int32 | undefined {
    return values.at((values.length as isize) - (1 + 1));
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-negative-index]: at index is expressed relative to array length
 ──▶ main.tspp:2:22
  │
1 │ function read(values: int32[]): int32 | undefined {
2 │     return values.at((values.length as isize) - (1 + 1));
  │                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use a negative index
--- a/main.tspp
+++ b/main.tspp

    1│ function read(values: int32[]): int32 | undefined {
-   2│     return values.at((values.length as isize) - (1 + 1));
+   2│     return values.at(-(1 + 1));
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function read(values: int32[]): int32 | undefined {
    return values.at(-(1 + 1));
}
"#,
        );
    }

    /// Preserve comments around the offset by omitting the fix.
    #[test]
    fn test_reports_commented_offset_without_fix() {
        let session = TestSession::dir(
            &PREFER_NEGATIVE_INDEX,
            r#"
function read(values: int32[]): int32 | undefined {
    return values.at((values.length as isize) - /* retain */ 2);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-negative-index]: at index is expressed relative to array length
 ──▶ main.tspp:2:22
  │
1 │ function read(values: int32[]): int32 | undefined {
2 │     return values.at((values.length as isize) - /* retain */ 2);
  │                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
