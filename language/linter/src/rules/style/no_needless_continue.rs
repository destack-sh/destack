use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow continue where control already proceeds to the same iteration.
    pub NO_NEEDLESS_CONTINUE {
        id: "no-needless-continue",
        summary: "Disallow continue where control already proceeds to the same iteration",
        explanation: r#"
A `continue` at the end of its target loop path repeats the control transfer already performed by ordinary completion.
Instead, you SHOULD remove the redundant statement and let the iteration finish normally.
"#,
        example: {
            reported: r#"
function visit(values: int32[]): void {
    for (const value of values) {
        value;
        continue;
    }
}
"#,
            accepted: r#"
function visit(values: int32[]): void {
    for (const value of values) {
        value;
    }
}
"#,
        },
        provenance: [Clippy("needless_continue")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report continues that already occupy the end of their target iteration path.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every authored continue with a target
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Continue { .. }) {
            continue;
        }
        let target = module.transfer_target(expression)?;
        let Some(body) = module.iteration_body(target) else {
            continue;
        };
        if !module.is_terminal_in(expression, body) {
            continue;
        }

        // remove the statement while preserving required bodies
        let span = module.span(expression.into_any())?;
        let patch = module.statement_removal_patch(expression)?;
        let suggestion = lint.fix("remove the redundant continue", patch)?;
        let diagnostic = lint
            .diagnostic("continue repeats the end of this iteration", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a continue at the end of a loop body.
    #[test]
    fn test_removes_terminal_continue() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        value;
        continue;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-needless-continue]: continue repeats the end of this iteration
 ──▶ main.tspp:4:9
  │
2 │     for (const value of values) {
3 │         value;
4 │         continue;
  │         ^^^^^^^^
5 │     }
6 │ }
  │

 = fix: remove the redundant continue
--- a/main.tspp
+++ b/main.tspp

    3│         value;
-   4│         continue;
    5│     }
"#,
        );
        session.assert_fixes(
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        value;
    }
}
"#,
        );
    }

    /// Remove a continue at the end of one conditional path.
    #[test]
    fn test_removes_terminal_conditional_continue() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            value;
            continue;
        }
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            value;
        }
    }
}
"#,
        );
    }

    /// Remove a continue from a terminal match arm.
    #[test]
    fn test_removes_terminal_match_continue() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        match (value) {
            0 => {
                value;
                continue;
            }
            _ => {
                value;
            }
        }
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        match (value) {
            0 => {
                value;
            }
            _ => {
                value;
            }
        }
    }
}
"#,
        );
    }

    /// Replace a direct terminal match-arm continue with a commented empty block.
    #[test]
    fn test_replaces_terminal_match_value_continue_with_block() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        match (value) {
            _ => continue
        }
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        match (value) {
            _ => { /* intentionally empty */ }
        }
    }
}
"#,
        );
    }

    /// Accept a continue that skips later work in the loop body.
    #[test]
    fn test_accepts_continue_before_work() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            continue;
        }
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a continue targeting an outer loop with remaining work.
    #[test]
    fn test_accepts_outer_continue_before_work() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(rows: int32[][]): void {
    outer: for (const row of rows) {
        for (const value of row) {
            continue outer;
        }
        row;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a continue that exits a nested loop before repeating the outer loop.
    #[test]
    fn test_accepts_continue_across_nested_loop() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            r#"
function visit(rows: int32[][]): void {
    outer: for (const row of rows) {
        for (const value of row) {
            value;
            continue outer;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
