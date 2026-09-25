use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow calls to the canonical todo function.
    pub NO_TODO {
        id: "no-todo",
        summary: "Disallow calls to the canonical todo function",
        explanation: r#"
A `todo` call marks unfinished code and traps if execution reaches it.
Instead, you SHOULD implement the operation or remove the path that requires it.
"#,
        example: {
            reported: r#"
import { todo } from "tspp:error";

function process(): void {
    todo("process");
}
"#,
            accepted: r#"
function process(): void {}
"#,
        },
        provenance: [Clippy("todo")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report calls to the canonical unfinished-code trap.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical todo calls
    for expression in module.call_expressions() {
        let expression = expression?;
        if module.language_item(expression)? != Some(dir::LanguageItem::Todo) {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("unfinished code can trap at runtime", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a canonical todo call.
    #[test]
    fn test_reports_todo_call() {
        let session = TestSession::dir(
            &NO_TODO,
            r#"
import { todo } from "tspp:error";

function process(): void {
    todo("process");
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-todo]: unfinished code can trap at runtime
 ──▶ main.tspp:4:5
  │
2 │
3 │ function process(): void {
4 │     todo("process");
  │     ^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept a user-defined function named todo.
    #[test]
    fn test_accepts_user_function() {
        let session = TestSession::dir(
            &NO_TODO,
            r#"
function todo(message: string): void {}

function process(): void {
    todo("process");
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
