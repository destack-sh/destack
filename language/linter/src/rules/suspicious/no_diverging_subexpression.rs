use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow diverging operations inside continuing expressions.
    pub NO_DIVERGING_SUBEXPRESSION {
        id: "no-diverging-subexpression",
        summary: "Disallow diverging operations inside continuing expressions",
        explanation: r#"
A diverging operand prevents the surrounding expression from reaching operations that follow it.
Instead, you SHOULD place the diverging operation in explicit control flow.
"#,
        example: {
            reported: r#"
declare function stop(): never;

function require(active: boolean): boolean {
    return active || stop();
}
"#,
            accepted: r#"
declare function stop(): never;

function require(active: boolean): boolean {
    if (!active) {
        stop();
    }
    return true;
}
"#,
        },
        provenance: [Clippy("diverging_sub_expression")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report diverging values embedded in another expression.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // select expressions nested within another value expression
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some(enclosing_expression) = module.enclosing_value_expression(expression) else {
            continue;
        };

        // select the outermost diverging operand of a continuing expression
        if !matches!(module.node_type(expression.into_any())?, dir::Type::Never)
            || matches!(
                module.node_type(enclosing_expression.into_any())?,
                dir::Type::Never
            )
        {
            continue;
        }

        // report the diverging operand
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("diverging operand prevents later evaluation", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a diverging right operand in a short-circuit expression.
    #[test]
    fn test_reports_short_circuit_operand() {
        let session = TestSession::dir(
            &NO_DIVERGING_SUBEXPRESSION,
            r#"
declare function stop(): never;

function require(active: boolean): boolean {
    return active || stop();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-diverging-subexpression]: diverging operand prevents later evaluation
 ──▶ main.tspp:4:22
  │
2 │
3 │ function require(active: boolean): boolean {
4 │     return active || stop();
  │                      ^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept divergence used as a complete statement.
    #[test]
    fn test_accepts_diverging_statement() {
        let session = TestSession::dir(
            &NO_DIVERGING_SUBEXPRESSION,
            r#"
declare function stop(): never;

function require(): void {
    stop();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept divergence used directly as a match-arm value.
    #[test]
    fn test_accepts_diverging_match_arm() {
        let session = TestSession::dir(
            &NO_DIVERGING_SUBEXPRESSION,
            r#"
declare function stop(): never;

function select(active: boolean): int32 {
    return match (active) {
        true => 1
        false => stop()
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report one diverging match used inside a continuing expression.
    #[test]
    fn test_reports_diverging_match_operand() {
        let session = TestSession::dir(
            &NO_DIVERGING_SUBEXPRESSION,
            r#"
declare function stop(): never;

function require(active: boolean): boolean {
    return active || match (active) {
        true => stop()
        false => stop()
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-diverging-subexpression]: diverging operand prevents later evaluation
 ──▶ main.tspp:4:22
  │
2 │
3 │ function require(active: boolean): boolean {
4 │     return active || match (active) {
  │                      ^^^^^^^^^^^^^^^^
5 │         true => stop()
  │         ^^^^^^^^^^^^^^
6 │         false => stop()
  │         ^^^^^^^^^^^^^^^
7 │     };
  │     ^
8 │ }
  │
"#,
        );
    }
}
