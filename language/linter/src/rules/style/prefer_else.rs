use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer else after a diverging if branch.
    pub PREFER_ELSE {
        id: "prefer-else",
        summary: "Prefer else after a diverging if branch",
        explanation: r#"
Statements after a diverging `if` branch run only when its condition is false.
Instead, you SHOULD place the remaining path in an `else` branch.
"#,
        example: {
            reported: r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    }
    return "nonnegative";
}
"#,
            accepted: r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    } else {
        return "nonnegative";
    }
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report diverging if branches followed by a separate remaining path.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect if statements followed by another expression in the same block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let mut expressions = block.iter_expressions().peekable();
        while let Some(expression) = expressions.next() {
            // require a remaining path after the conditional
            if expressions.peek().is_none() {
                break;
            }

            // select a regular if with one diverging branch
            let dir::Expression::If {
                form: dir::IfForm::If,
                then_expression,
                else_expression: None,
                ..
            } = view.get(expression)
            else {
                continue;
            };
            if !module.is_diverging(then_expression.into_any())? {
                continue;
            }

            // report the separate remaining path
            let span = module.main_span(expression.into_any())?;
            output.report(lint.diagnostic("remaining path is separate from its if", span));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an alternative after a call-diverging branch.
    #[test]
    fn test_reports_call_diverging_branch() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function classifyNonzero(value: int32): never;

function classify(value: int32): string {
    if (value !== 0) {
        classifyNonzero(value);
    }
    return "zero";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-else]: remaining path is separate from its if
 ──▶ main.ds:4:5
  │
2 │
3 │ function classify(value: int32): string {
4 │     if (value !== 0) {
  │     ^^
5 │         classifyNonzero(value);
6 │     }
  │
"#,
        );
    }

    /// Report every branch in one adjacent terminal decision chain.
    #[test]
    fn test_reports_terminal_chain() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
function classify(value: int32): string {
    if (value < 0) {
        return "negative";
    }
    if (value > 0) {
        return "positive";
    }
    return "zero";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-else]: remaining path is separate from its if
 ──▶ main.ds:2:5
  │
1 │ function classify(value: int32): string {
2 │     if (value < 0) {
  │     ^^
3 │         return "negative";
4 │     }
  │

warning[prefer-else]: remaining path is separate from its if
 ──▶ main.ds:5:5
  │
3 │         return "negative";
4 │     }
5 │     if (value > 0) {
  │     ^^
6 │         return "positive";
7 │     }
  │
"#,
        );
    }

    /// Report a returning branch before a continuing function path.
    #[test]
    fn test_reports_return_before_continuation() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function work(): void;

function run(isInvalid: boolean): void {
    if (isInvalid) {
        return;
    }
    work();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-else]: remaining path is separate from its if
 ──▶ main.ds:4:5
  │
2 │
3 │ function run(isInvalid: boolean): void {
4 │     if (isInvalid) {
  │     ^^
5 │         return;
6 │     }
  │
"#,
        );
    }

    /// Report a continuing branch before the remaining iteration path.
    #[test]
    fn test_reports_continue_before_iteration_path() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function work(value: int32): void;

function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            continue;
        }
        work(value);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-else]: remaining path is separate from its if
 ──▶ main.ds:5:9
  │
3 │ function visit(values: int32[]): void {
4 │     for (const value of values) {
5 │         if (value < 0) {
  │         ^^
6 │             continue;
7 │         }
  │
"#,
        );
    }

    /// Accept an implicit undefined result in a default interface method.
    #[test]
    fn test_accepts_default_interface_expression() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
newtype interface Source {
    source(): int32 | undefined {
        undefined
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
