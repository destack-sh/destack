use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow debugger statements.
    pub NO_DEBUGGER {
        id: "no-debugger",
        summary: "Disallow debugger statements",
        explanation: r#"
The `debugger` statement pauses execution when an attached debugger reaches it.
Instead, you SHOULD remove the statement and set a breakpoint through the debugger when needed.
"#,
        example: {
            reported: r#"
declare const active: boolean;
if (active) {
    const resumed = true;
    debugger;
}
"#,
            accepted: r#"
declare const active: boolean;
if (active) {
    const resumed = true;
}
"#,
        },
        provenance: [Eslint("no-debugger")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report debugger statements.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // report every visible debugger expression
    for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
        // skip other expressions
        if !matches!(expression, dir::Expression::Debugger) {
            continue;
        }

        let span = module.span(expression_id.into_any())?;
        let patch = module.statement_removal_patch(expression_id)?;
        let suggestion = lint.fix("remove the debugger statement", patch)?;
        let diagnostic = lint
            .diagnostic("`debugger` statement is not allowed", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Suppress the lint through its canonical id.
    #[test]
    fn test_allows_debugger_by_id() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
@allow("no-debugger")
debugger;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove the debugger statement without changing surrounding source.
    #[test]
    fn test_removes_debugger_statement() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
const before = 1;
debugger;
const after = 2;
"#,
        );

        session.assert_fixes(
            r#"const before = 1;

const after = 2;
"#,
        );
    }

    /// Remove a debugger statement from an explicit block.
    #[test]
    fn test_removes_debugger_from_block() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
declare const active: boolean;
if (active) {
    debugger;
}
"#,
        );

        session.assert_fixes(
            r#"declare const active: boolean;
if (active) {
}
"#,
        );
    }

    /// Preserve the required body of a catch clause.
    #[test]
    fn test_replaces_catch_body() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
try {
} catch (error) debugger
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-debugger]: `debugger` statement is not allowed
 ──▶ main.tspp:2:17
  │
1 │ try {
2 │ } catch (error) debugger
  │                 ^^^^^^^^
  │

 = fix: remove the debugger statement
--- a/main.tspp
+++ b/main.tspp

    1│ try {
-   2│ } catch (error) debugger
+   2│ } catch (error) { /* intentionally empty */ }
"#,
        );
        session.assert_fixes(
            r#"try {
} catch (error) { /* intentionally empty */ }
"#,
        );
    }

    /// Preserve the required body of a finally clause.
    #[test]
    fn test_replaces_finally_body() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
try {
} finally debugger
"#,
        );

        session.assert_fixes(
            r#"try {
} finally { /* intentionally empty */ }
"#,
        );
    }

    /// Remove the debugger statement from a switch case.
    #[test]
    fn test_removes_switch_case_statement() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
switch (1) {
    case 1:
        debugger;
}
"#,
        );

        session.assert_fixes(
            r#"switch (1) {
    case 1:
}
"#,
        );
    }

    /// Preserve the required value of a direct match arm.
    #[test]
    fn test_replaces_match_arm() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
match (undefined) {
    _ => debugger
}
"#,
        );

        session.assert_fixes(
            r#"match (undefined) {
    _ => { /* intentionally empty */ }
}
"#,
        );
    }

    /// Ignore property declarations and accesses named debugger.
    #[test]
    fn test_ignores_debugger_property() {
        let session = TestSession::dir(
            &NO_DEBUGGER,
            r#"
const value = { debugger: true };
value.debugger;
"#,
        );

        session.assert_no_diagnostics();
    }
}
