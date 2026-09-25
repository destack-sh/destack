use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assignment within an explicit or implicit return value.
    pub NO_RETURN_ASSIGN {
        id: "no-return-assign",
        summary: "Disallow assignment within an explicit or implicit return value",
        explanation: r#"
A returned assignment makes the assigned value the function result after mutating its target, and a single `=` can be mistaken for an equality operator.
Instead, you SHOULD perform the assignment as a statement and return the resulting value separately.
"#,
        example: {
            reported: r#"
function reset(value: int32): int32 {
    let current = value;
    return (current = 0);
}
"#,
            accepted: r#"
function reset(value: int32): int32 {
    let current = value;
    current = 0;
    return current;
}
"#,
        },
        provenance: [Eslint("no-return-assign")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report assignments nested within explicit or implicit return values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every authored assignment expression
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Assign { .. })
            || !module.is_within_return_value(expression)
        {
            continue;
        }

        // report the complete grouped assignment
        let span = match module.source_parentheses(expression.into_any()) {
            Some(parentheses) => parentheses,
            None => module.source_extent(expression.into_any())?,
        };
        let diagnostic = lint.diagnostic("return value is an assignment", span);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an assignment followed by a separate return.
    #[test]
    fn test_accepts_separate_assignment() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
function reset(value: int32): int32 {
    let current = value;
    current = 0;
    return current;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an assignment returned by an expression-bodied lambda.
    #[test]
    fn test_reports_implicit_assignment() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
let current = 1;
const reset: () => int32 = (): int32 => (current = 0);
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-return-assign]: return value is an assignment
 ──▶ main.tspp:2:41
  │
1 │ let current = 1;
2 │ const reset: () => int32 = (): int32 => (current = 0);
  │                                         ^^^^^^^^^^^^^
  │
"#,
        );
    }

    /// Report an assignment nested within an explicit return expression.
    #[test]
    fn test_reports_nested_return_assignment() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
function reset(isActive: boolean): int32 {
    let current: int32 = 1;
    return isActive ? (current = 0) : current;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-return-assign]: return value is an assignment
 ──▶ main.tspp:3:23
  │
1 │ function reset(isActive: boolean): int32 {
2 │     let current: int32 = 1;
3 │     return isActive ? (current = 0) : current;
  │                       ^^^^^^^^^^^^^
4 │ }
  │
"#,
        );
    }

    /// Report an assignment nested within an implicitly returned object.
    #[test]
    fn test_reports_assignment_in_returned_object() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
let current = 1;
const reset = () => ({ value: (current = 0) });
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-return-assign]: return value is an assignment
 ──▶ main.tspp:2:31
  │
1 │ let current = 1;
2 │ const reset = () => ({ value: (current = 0) });
  │                               ^^^^^^^^^^^^^
  │
"#,
        );
    }
}
