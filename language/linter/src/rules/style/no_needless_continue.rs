use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
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
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Reject this lint until DIR carries checked control flow.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} requires checked control flow in DIR",
        lint.id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Validate the canonical lint example.
    #[ignore]
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&NO_NEEDLESS_CONTINUE);
    }

    /// Remove a continue at the end of a loop body.
    #[ignore]
    #[test]
    fn test_removes_terminal_continue() {
        let session = TestSession::dir(
            &NO_NEEDLESS_CONTINUE,
            NO_NEEDLESS_CONTINUE.example.reported(),
        );

        session.assert_diagnostics(
            r#"
warning[no-needless-continue]: continue repeats the end of this iteration
 ──▶ main.ds:4:9
  │
2 │     for (const value of values) {
3 │         value;
4 │         continue;
  │         ^^^^^^^^
5 │     }
6 │ }
  │

 = fix: remove the redundant continue
--- a/main.ds
+++ b/main.ds

    3│         value;
-   4│         continue;
"#,
        );
        session.assert_fixes(NO_NEEDLESS_CONTINUE.example.accepted());
    }

    /// Remove a continue at the end of one conditional path.
    #[ignore]
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

    /// Accept a continue that skips later work in the loop body.
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
