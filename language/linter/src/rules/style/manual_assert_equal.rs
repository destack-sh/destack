use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer equality assertions over asserting a comparison.
    pub MANUAL_ASSERT_EQUAL {
        id: "manual-assert-equal",
        summary: "Prefer equality assertions over asserting a comparison",
        explanation: r#"
A boolean assertion receives only the result of an equality comparison, so a failure cannot report
the actual and expected values.
Instead, you SHOULD use `assertEqual` or `assertNotEqual` so both values are available to the
diagnostic.
"#,
        example: {
            reported: r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assert(actual == expected);
}
"#,
            accepted: r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assertEqual(actual, expected);
}
"#,
        },
        provenance: [Clippy("manual_assert_eq")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical boolean assertions containing one value equality comparison.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect calls to the canonical boolean assertion
    for expression in module.call_expressions() {
        let expression = expression?;
        if module.language_item(expression)? != Some(dir::LanguageItem::Assert) {
            continue;
        }
        let dir::Expression::Call {
            generic_arguments,
            arguments,
            is_optional: false,
            ..
        } = view.get(expression)
        else {
            continue;
        };
        if !generic_arguments.is_empty() || !(1..=2).contains(&arguments.len()) {
            continue;
        }
        let Some(condition) = view.get(arguments[0]).value() else {
            continue;
        };
        let dir::Expression::Binary {
            operator,
            left,
            right,
        } = view.get(condition)
        else {
            continue;
        };
        let assertion = match operator {
            dir::BinaryOperator::Equal => "assertEqual",
            dir::BinaryOperator::NotEqual => "assertNotEqual",
            _ => continue,
        };

        // report the comparison and retain authored operands in any rewrite
        let span = module.source_extent(condition.into_any())?;
        let mut diagnostic = lint.diagnostic("boolean assertion contains an equality test", span);
        if let Some(suggestion) = suggestion(
            module, lint, expression, condition, *left, *right, assertion,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Suggest the corresponding equality assertion when its namespace is retained.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
    assertion: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };

    // retain authored grouping around the comparison and both operands
    let condition = grouped_span(module, condition)?;
    let left = grouped_span(module, left)?;
    let right = grouped_span(module, right)?;
    if module.has_unretained_comment(condition, &[left, right])? {
        return Ok(None);
    }

    // replace the assertion member and split the comparison operands
    let callee = module.main_span(call.callee.into_any())?;
    let left = module.source(left)?;
    let right = module.source(right)?;
    let mut file = FilePatch::new(callee.file);
    file.replace(callee, assertion);
    file.replace(condition, format!("{left}, {right}"));
    let suggestion = lint.suggestion(format!("use `{assertion}`"), file)?;

    Ok(Some(suggestion))
}

/// Return one expression span including its authored parentheses.
fn grouped_span(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Span, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;

    Ok(module
        .source_parentheses(expression.into_any())
        .unwrap_or(extent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace inequality while retaining an authored message.
    #[test]
    fn test_replaces_inequality_with_message() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assert(actual != expected, "values must differ");
}
"#,
        );

        session.assert_suggestions(
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assertNotEqual(actual, expected, "values must differ");
}
"#,
        );
    }

    /// Replace a parenthesized comparison without retaining tuple grouping.
    #[test]
    fn test_replaces_parenthesized_equality() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assert((actual == expected));
}
"#,
        );

        session.assert_suggestions(
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assertEqual(actual, expected);
}
"#,
        );
    }

    /// Report a directly imported assertion without inventing another import.
    #[test]
    fn test_reports_direct_import_without_suggestion() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
import { assert } from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert(actual == expected);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-assert-equal]: boolean assertion contains an equality test
 ──▶ main.tspp:4:12
  │
2 │
3 │ function verify(actual: int32, expected: int32): void {
4 │     assert(actual == expected);
  │            ^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Report a commented comparison without discarding its comment.
    #[test]
    fn test_reports_commented_equality_without_suggestion() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assert(actual /* retain */ == expected);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-assert-equal]: boolean assertion contains an equality test
 ──▶ main.tspp:4:19
  │
2 │
3 │ function verify(actual: int32, expected: int32): void {
4 │     assert.assert(actual /* retain */ == expected);
  │                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept strict comparisons without a corresponding strict assertion API.
    #[test]
    fn test_accepts_strict_equality() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assert(actual === expected);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept assertions around other predicates.
    #[test]
    fn test_accepts_other_predicate() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
import * as assert from "tspp:assert";

function verify(actual: int32, expected: int32): void {
    assert.assert(actual < expected);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined assertion functions.
    #[test]
    fn test_accepts_user_assert() {
        let session = TestSession::dir(
            &MANUAL_ASSERT_EQUAL,
            r#"
declare function assert(condition: boolean): void;

function verify(actual: int32, expected: int32): void {
    assert(actual == expected);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
