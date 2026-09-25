use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow chained assignment.
    pub NO_MULTI_ASSIGN {
        id: "no-multi-assign",
        summary: "Disallow chained assignment",
        explanation: r#"
A chained assignment is right-associative, so `left = right = value` writes `right` before assigning that result to `left`.
Instead, you SHOULD use separate assignments in the same order.
"#,
        example: {
            reported: r#"
function reset(): void {
    let left = 1;
    let right = 2;
    left = right = 0;
}
"#,
            accepted: r#"
function reset(): void {
    let left = 1;
    let right = 2;
    right = 0;
    left = right;
}
"#,
        },
        provenance: [Eslint("no-multi-assign")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report assignments whose value is another assignment.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect assignment values
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Assign { right, .. } = node else {
            continue;
        };

        // require another assignment as the value
        if !matches!(view.get(*right), dir::Expression::Assign { .. }) {
            continue;
        }

        // report only the outermost assignment in a chain
        if matches!(
            view.ancestor::<dir::Expression>(expression.into_any())
                .map(|parent| view.get(parent)),
            Some(dir::Expression::Assign { right, .. }) if *right == expression
        ) {
            continue;
        }

        // report the complete assignment chain
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("assignment is chained", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept separate assignments.
    #[test]
    fn test_accepts_separate_assignments() {
        let session = TestSession::dir(
            &NO_MULTI_ASSIGN,
            r#"
function reset(): void {
    let left = 1;
    let right = 2;
    right = 0;
    left = right;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report one diagnostic for an assignment chain of any length.
    #[test]
    fn test_reports_assignment_chain_once() {
        let session = TestSession::dir(
            &NO_MULTI_ASSIGN,
            r#"
function reset(): void {
    let first = 1;
    let second = 2;
    let third = 3;
    first = second = third = 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-multi-assign]: assignment is chained
 ──▶ main.tspp:5:5
  │
3 │     let second = 2;
4 │     let third = 3;
5 │     first = second = third = 0;
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │ }
  │
"#,
        );
    }
}
