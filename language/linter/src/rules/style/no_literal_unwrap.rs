use tspp_dir as dir;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow immediately unwrapping a known successful result variant.
    pub NO_LITERAL_UNWRAP {
        id: "no-literal-unwrap",
        summary: "Disallow immediately unwrapping a known successful result variant",
        explanation: r#"
Unwrapping a newly constructed successful result returns the payload supplied to its constructor.
Instead, you SHOULD use that payload directly.
"#,
        example: {
            reported: r#"
function identity(value: int32): int32 {
    return Result<int32, string>.ok(value).unwrap();
}
"#,
            accepted: r#"
function identity(value: int32): int32 {
    return value;
}
"#,
        },
        provenance: [Clippy("unnecessary_literal_unwrap")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report successful unwraps applied directly to canonical `Result` constructors.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let result = dir::LanguageItem::Result;
    let unwrap = result.member("unwrap");
    let unwrap_err = result.member("unwrapErr");
    let ok = result.member("ok");
    let err = result.member("err");
    let mut output = LintOutput::default();

    // inspect selected zero-argument unwrap calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if !call.arguments.is_empty() {
            continue;
        }
        let Some(operation) = module.language_member(expression)? else {
            continue;
        };
        if operation != unwrap && operation != unwrap_err {
            continue;
        }

        // select the canonical result constructor receiver
        let dir::Expression::Call {
            arguments: constructor_arguments,
            ..
        } = view.get(call.receiver)
        else {
            continue;
        };
        let [argument] = constructor_arguments.as_slice() else {
            continue;
        };
        let Some(value) = view.get(*argument).value() else {
            continue;
        };
        let Some(constructor) = module.language_member(call.receiver)? else {
            continue;
        };
        let is_success = (operation == unwrap && constructor == ok)
            || (operation == unwrap_err && constructor == err);
        if !is_success {
            continue;
        }

        // replace the redundant construction and unwrap when comments survive
        let span = module.source_extent(expression.into_any())?;
        let value_span = module.source_extent(value.into_any())?;
        let mut diagnostic =
            lint.diagnostic("result is unwrapped immediately after construction", span);
        if !module.has_unretained_comment(span, &[value_span])? {
            let replacement = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
            let patch = Patch::replace(span, replacement);
            let preserves_type = module.node_type_id(expression.into_any())?
                == module.node_type_id(value.into_any())?;
            let suggestion = if preserves_type {
                lint.fix("use the constructed value directly", patch)?
            } else {
                lint.suggestion("use the constructed value directly", patch)?
            };
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a redundant failed-value unwrap with its error.
    #[test]
    fn test_replaces_err_unwrap_err() {
        let session = TestSession::dir(
            &NO_LITERAL_UNWRAP,
            r#"
const error: string = Result<never, "failure">.err("failure").unwrapErr();
"#,
        );

        session.assert_fixes(
            r#"
const error: string = "failure";
"#,
        );
    }

    /// Preserve grouping when the constructed value has lower precedence.
    #[test]
    fn test_groups_binary_replacement() {
        let session = TestSession::dir(
            &NO_LITERAL_UNWRAP,
            r#"
declare const left: int32;
declare const right: int32;
declare const factor: int32;
const value = Result<int32, string>.ok(left + right).unwrap() * factor;
"#,
        );

        session.assert_fixes(
            r#"
declare const left: int32;
declare const right: int32;
declare const factor: int32;
const value = (left + right) * factor;
"#,
        );
    }

    /// Require review when removing the unwrap changes the inferred static type.
    #[test]
    fn test_suggests_literal_replacement() {
        let session = TestSession::dir(
            &NO_LITERAL_UNWRAP,
            r#"
const value = Result<int32, string>.ok(42).unwrap();
"#,
        );

        session.assert_suggestions(
            r#"
const value = 42;
"#,
        );
    }

    /// Leave a statically failing unwrap to the dedicated correctness rule.
    #[test]
    fn test_ignores_err_unwrap() {
        let session = TestSession::dir(
            &NO_LITERAL_UNWRAP,
            r#"
const value: int32 = Result<int32, "failure">.err("failure").unwrap();
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an unwrap whose receiver is not constructed in place.
    #[test]
    fn test_accepts_stored_result() {
        let session = TestSession::dir(
            &NO_LITERAL_UNWRAP,
            r#"
declare const result: Result<int32, string>;
const value = result.unwrap();
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a correction when replacement would discard a comment.
    #[test]
    fn test_preserves_constructor_comment() {
        let session = TestSession::dir(
            &NO_LITERAL_UNWRAP,
            r#"
const value: int32 = Result<int32, string>.ok(/* retained */ 42).unwrap();
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-literal-unwrap]: result is unwrapped immediately after construction
 ──▶ main.tspp:1:22
  │
1 │ const value: int32 = Result<int32, string>.ok(/* retained */ 42).unwrap();
  │                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │
"#,
        );
    }
}
