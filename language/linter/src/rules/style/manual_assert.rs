use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer assertions over conditional panics.
    pub MANUAL_ASSERT {
        id: "manual-assert",
        summary: "Prefer assertions over conditional panics",
        explanation: r#"
An `if` branch whose only operation is `panic` expresses an assertion through manual control flow.
Instead, you SHOULD use `assert` with the opposite condition.

Wrap a computed message in a zero-argument lambda so it is evaluated only after failure.
"#,
        example: {
            reported: r#"
import { panic } from "tspp:error";

function divide(value: int32, divisor: int32): int32 {
    if (divisor == 0) {
        panic("divisor must not be zero");
    }

    return value / divisor;
}
"#,
            accepted: r#"
import { assert } from "tspp:assert";

function divide(value: int32, divisor: int32): int32 {
    assert(divisor != 0, "divisor must not be zero");

    return value / divisor;
}
"#,
        },
        provenance: [Clippy("manual_assert")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report conditional branches whose only operation is a canonical panic.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect regular if statements without an alternate branch
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition,
            then_expression,
            else_expression: None,
        } = node
        else {
            continue;
        };
        if condition.as_expression().is_none() {
            continue;
        }

        // require one canonical panic call as the complete branch body
        let Some(panic) = module.sole_expression(*then_expression) else {
            continue;
        };
        let dir::Expression::Call {
            is_optional: false, ..
        } = view.get(panic)
        else {
            continue;
        };
        if module.language_item(panic)? != Some(dir::LanguageItem::Panic) {
            continue;
        }

        // report the complete manual assertion
        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("conditional branch only panics", span)
            .help("replace the branch with an assertion of the opposite condition");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a panic guarded by an existing negation.
    #[test]
    fn test_reports_negated_condition() {
        let session = TestSession::dir(
            &MANUAL_ASSERT,
            r#"
import { panic } from "tspp:error";

function requireReady(isReady: boolean): void {
    if (!isReady) {
        panic("service must be ready");
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-assert]: conditional branch only panics
 ──▶ main.tspp:4:5
  │
2 │
3 │ function requireReady(isReady: boolean): void {
4 │     if (!isReady) {
  │     ^^^^^^^^^^^^^^^
5 │         panic("service must be ready");
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │     }
  │     ^
7 │ }
  │

 = help: replace the branch with an assertion of the opposite condition
"#,
        );
    }

    /// Report an unbraced panic with a computed message.
    #[test]
    fn test_reports_unbraced_panic() {
        let session = TestSession::dir(
            &MANUAL_ASSERT,
            r#"
import { panic } from "tspp:error";

declare function describeFailure(): string;

function requireReady(isReady: boolean): void {
    if (!isReady) panic(describeFailure());
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-assert]: conditional branch only panics
 ──▶ main.tspp:6:5
  │
4 │
5 │ function requireReady(isReady: boolean): void {
6 │     if (!isReady) panic(describeFailure());
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │ }
  │

 = help: replace the branch with an assertion of the opposite condition
"#,
        );
    }

    /// Accept an assertion with a lazily computed message.
    #[test]
    fn test_accepts_lazy_assertion_message() {
        let session = TestSession::dir(
            &MANUAL_ASSERT,
            r#"
import { assert } from "tspp:assert";

declare function describeFailure(): string;

function requireReady(isReady: boolean): void {
    assert(isReady, () => describeFailure());
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a branch that performs another operation before panicking.
    #[test]
    fn test_accepts_branch_with_other_work() {
        let session = TestSession::dir(
            &MANUAL_ASSERT,
            r#"
import { panic } from "tspp:error";

declare function trace(message: string): void;

function requireReady(isReady: boolean): void {
    if (!isReady) {
        trace("not ready");
        panic("service must be ready");
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a panic branch with an alternate path.
    #[test]
    fn test_accepts_else_branch() {
        let session = TestSession::dir(
            &MANUAL_ASSERT,
            r#"
import { panic } from "tspp:error";

function requireReady(isReady: boolean): void {
    if (!isReady) {
        panic("service must be ready");
    } else {
        return;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept calls to user-defined panic functions.
    #[test]
    fn test_accepts_user_panic() {
        let session = TestSession::dir(
            &MANUAL_ASSERT,
            r#"
declare function panic(message: string): void;

function requireReady(isReady: boolean): void {
    if (!isReady) {
        panic("service must be ready");
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
