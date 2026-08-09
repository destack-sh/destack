use destack_dir as dir;
use destack_source::NodeSpanRegion;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow else branches after an unconditional control transfer.
    pub NO_ELSE_RETURN {
        id: "no-else-return",
        summary: "Disallow else branches after an unconditional control transfer",
        explanation: r#"
An else branch is redundant when the preceding branch cannot continue. End the transferring branch,
then place the remaining path after the if so the main flow stays unnested.
"#,
        example: {
            reported: r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    } else {
        return "nonnegative";
    }
}
"#,
            accepted: r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    }
    return "nonnegative";
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report else branches whose preceding branch has checked `never` type.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect regular if expressions with a continuing alternative
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            then_expression,
            else_expression: Some(_),
            ..
        } = node
        else {
            continue;
        };
        if !matches!(
            module.node_type(then_expression.into_any())?,
            dir::Type::Never
        ) {
            continue;
        }

        // report the redundant alternative keyword
        let span = module.source_region(expression.into_any(), NodeSpanRegion::Else)?;
        output.report(lint.diagnostic("else follows an unconditional control transfer", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an else branch when its preceding branch can continue.
    #[test]
    fn test_accepts_continuing_then_branch() {
        let session = TestSession::dir(
            &NO_ELSE_RETURN,
            r#"
declare function record(): void;
function recordSign(value: int32): void {
    if (value < 0) {
        record();
    } else {
        record();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report after a branch whose nested alternatives both return.
    #[test]
    fn test_reports_after_nested_returning_branch() {
        let session = TestSession::dir(
            &NO_ELSE_RETURN,
            r#"
function classify(value: int32): string {
    if (value !== 0) {
        if (value < 0) {
            return "negative";
        }
        return "positive";
    } else {
        return "zero";
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-else-return]: else follows an unconditional control transfer
 ──▶ main.ds:7:7
  │
5 │         }
6 │         return "positive";
7 │     } else {
  │       ^^^^
8 │         return "zero";
9 │     }
  │
"#,
        );
    }

    /// Report an else branch after an unconditional continue.
    #[test]
    fn test_reports_else_after_continue() {
        let session = TestSession::dir(
            &NO_ELSE_RETURN,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            continue;
        } else {
            return;
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-else-return]: else follows an unconditional control transfer
 ──▶ main.ds:5:11
  │
3 │         if (value < 0) {
4 │             continue;
5 │         } else {
  │           ^^^^
6 │             return;
7 │         }
  │
"#,
        );
    }

    /// Report an else branch after an unconditional break.
    #[test]
    fn test_reports_else_after_break() {
        let session = TestSession::dir(
            &NO_ELSE_RETURN,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            break;
        } else {
            return;
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-else-return]: else follows an unconditional control transfer
 ──▶ main.ds:5:11
  │
3 │         if (value < 0) {
4 │             break;
5 │         } else {
  │           ^^^^
6 │             return;
7 │         }
  │
"#,
        );
    }
}
