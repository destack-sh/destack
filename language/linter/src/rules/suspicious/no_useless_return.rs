use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
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
        provenance: [Eslint("no-useless-return")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report bare returns on terminal paths through their callable body.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every authored bare return
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Return { value: None }) {
            continue;
        }
        let Some(body) = module.enclosing_callable_body(expression.into_any()) else {
            continue;
        };
        let dir::Expression::Block(body) = view.get(body) else {
            continue;
        };
        if !module.is_terminal_in(expression, *body) {
            continue;
        }

        // remove the statement while preserving required bodies
        let span = module.span(expression.into_any())?;
        let patch = module.statement_removal_patch(expression)?;
        let suggestion = lint.fix("remove the redundant return", patch)?;
        let diagnostic = lint
            .diagnostic("return is redundant at the end of this function", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Remove a bare return at the end of a function body.
    #[test]
    fn test_removes_final_return() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    value;
    return;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-return]: return is redundant at the end of this function
 ──▶ main.tspp:3:5
  │
1 │ function record(value: int32): void {
2 │     value;
3 │     return;
  │     ^^^^^^
4 │ }
  │

 = fix: remove the redundant return
--- a/main.tspp
+++ b/main.tspp

    2│     value;
-   3│     return;
    4│ }
"#,
        );
        session.assert_fixes(
            r#"
function record(value: int32): void {
    value;
}
"#,
        );
    }

    /// Remove a bare return from a final conditional path.
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

    /// Remove a bare return from the final try body of a function.
    #[test]
    fn test_removes_return_from_final_try() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    try {
        value;
        return;
    } finally {
        value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function record(value: int32): void {
    try {
        value;
    } finally {
        value;
    }
}
"#,
        );
    }

    /// Accept a return in finally because it may override an earlier transfer.
    #[test]
    fn test_accepts_return_from_finally() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function record(value: int32): void {
    try {
        value;
    } finally {
        return;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a return that skips later function work.
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

    /// Accept a return used as the selected value of a terminal match.
    #[test]
    fn test_accepts_return_from_match_value() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function stop(): int32 {
    match (return 1) {
        _ => 0
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a direct terminal match-arm return with a commented empty block.
    #[test]
    fn test_replaces_terminal_match_value_return_with_block() {
        let session = TestSession::dir(
            &NO_USELESS_RETURN,
            r#"
function stop(value: int32): void {
    match (value) {
        _ => return
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function stop(value: int32): void {
    match (value) {
        _ => { /* intentionally empty */ }
    }
}
"#,
        );
    }

    /// Remove a bare return from the final switch case at function end.
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
