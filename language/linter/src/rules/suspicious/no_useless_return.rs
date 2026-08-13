use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow bare returns where the function already ends.
    pub NO_USELESS_RETURN {
        id: "no-useless-return",
        summary: "Disallow bare returns where the function already ends",
        explanation: r#"
A bare `return` on a path that already reaches the end of its function performs no additional control transfer.
Instead, you SHOULD remove the redundant statement and let the function complete normally.
"#,
        example: {
            reported: r#"
function record(value: int32): void {
    value;
    return;
}
"#,
            accepted: r#"
function record(value: int32): void {
    value;
}
"#,
        },
        category: Suspicious,
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
        TestSession::assert_example(&NO_USELESS_RETURN);
    }

    /// Remove a bare return at the end of a function body.
    #[ignore]
    #[test]
    fn test_removes_final_return() {
        let session = TestSession::dir(&NO_USELESS_RETURN, NO_USELESS_RETURN.example.reported());

        session.assert_diagnostics(
            r#"
warning[no-useless-return]: return is redundant at the end of this function
 ──▶ main.ds:3:5
  │
1 │ function record(value: int32): void {
2 │     value;
3 │     return;
  │     ^^^^^^
4 │ }
  │

 = fix: remove the redundant return
--- a/main.ds
+++ b/main.ds

    2│     value;
-   3│     return;
"#,
        );
        session.assert_fixes(NO_USELESS_RETURN.example.accepted());
    }

    /// Remove a bare return from a final conditional path.
    #[ignore]
    #[test]
    fn test_removes_final_conditional_return() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    if (value > 0) {
        value;
        return;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function record(value: int32): void {
    if (value > 0) {
        value;
    }
}
"#,
        );
    }

    /// Accept a return that skips later function work.
    #[ignore]
    #[test]
    fn test_accepts_return_before_work() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    if (value < 0) {
        return;
    }
    value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a return that exits an iteration.
    #[ignore]
    #[test]
    fn test_accepts_return_from_loop() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(values: int32[]): void {
    for (const value of values) {
        value;
        return;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove a bare return from the final switch case at function end.
    #[ignore]
    #[test]
    fn test_removes_return_from_final_switch_case() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    switch (value) {
        default:
            value;
            return;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function record(value: int32): void {
    switch (value) {
        default:
            value;
    }
}
"#,
        );
    }

    /// Accept a return that prevents fallthrough into another switch case.
    #[ignore]
    #[test]
    fn test_accepts_return_before_later_switch_case() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    switch (value) {
        case 0:
            return;
        default:
            value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a return that produces a value.
    #[ignore]
    #[test]
    fn test_accepts_return_value() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function identity(value: int32): int32 {
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
