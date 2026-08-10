use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assigning a stable place to itself.
    pub NO_SELF_ASSIGNMENT {
        id: "no-self-assignment",
        summary: "Disallow assigning a stable place to itself",
        explanation: r#"
Assigning a stable storage place to itself leaves its value unchanged and performs a useless write.
Instead, you SHOULD remove the assignment or correct the unintended operand.
"#,
        example: {
            reported: r#"
function retain(value: int32): int32 {
    let result = value;
    result = result;
    return result;
}
"#,
            accepted: r#"
function retain(value: int32): int32 {
    const result = value;
    return result;
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report direct assignments that read and write one stable place.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect plain assignments to direct places
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Assign {
            left,
            operator: dir::AssignOperator::Assign,
            right,
        } = node
        else {
            continue;
        };
        let dir::AssignPattern::Place { expression: left } = view.get(*left) else {
            continue;
        };

        // require the compiler to select one identical stable storage path
        let Some(left) = module.access_resolution(*left) else {
            continue;
        };
        let Some(right) = module.access_resolution(*right) else {
            continue;
        };
        if left != right {
            continue;
        }

        let span = module.span(expression.into_any())?;
        output.report(lint.diagnostic("assignment writes a value back to the same place", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a stored field self-assignment.
    #[test]
    fn test_reports_field_self_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
struct Point {
    x: int32;
}
function retain(point: Point): void {
    point.x = point.x;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-assignment]: assignment writes a value back to the same place
 ──▶ main.ds:5:5
  │
3 │ }
4 │ function retain(point: Point): void {
5 │     point.x = point.x;
  │     ^^^^^^^^^^^^^^^^^
6 │ }
  │
"#,
        );
    }

    /// Accept assignment from a distinct binding.
    #[test]
    fn test_accepts_distinct_binding_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
function replace(current: int32, next: int32): int32 {
    let result = current;
    result = next;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept compound assignment because it computes a new value.
    #[test]
    fn test_accepts_compound_self_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
function double(value: int32): int32 {
    let result = value;
    result += result;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
