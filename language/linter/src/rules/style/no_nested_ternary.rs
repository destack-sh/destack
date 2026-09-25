use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow nested ternary expressions.
    pub NO_NESTED_TERNARY {
        id: "no-nested-ternary",
        summary: "Disallow nested ternary expressions",
        explanation: r#"
A ternary nested directly inside another ternary forces multiple conditions and results into a grouping determined by operator associativity.
Instead, you SHOULD use explicit control flow for each condition and result.
"#,
        example: {
            reported: r#"
function classify(value: int32): string {
    return value > 0 ? "positive" : value < 0 ? "negative" : "zero";
}
"#,
            accepted: r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    }
    if (value < 0) {
        return "negative";
    }
    return "zero";
}
"#,
        },
        provenance: [Eslint("no-nested-ternary")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report ternaries nested directly inside another ternary.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect ternaries whose nearest expression parent is another ternary
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(
            node,
            dir::Expression::If {
                form: dir::IfForm::Ternary,
                ..
            }
        ) {
            continue;
        }

        // require another ternary as the nearest expression parent
        let Some(parent) = view.ancestor::<dir::Expression>(expression.into_any()) else {
            continue;
        };
        if !matches!(
            view.get(parent),
            dir::Expression::If {
                form: dir::IfForm::Ternary,
                ..
            }
        ) {
            continue;
        }

        // report the nested expression
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("ternary is nested inside another ternary", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept separate ternaries in separate statements.
    #[test]
    fn test_accepts_separate_ternaries() {
        let session = TestSession::dir(
            &NO_NESTED_TERNARY,
            r#"
function first(value: boolean): int32 {
    return value ? 1 : 0;
}
function second(value: boolean): int32 {
    return value ? 2 : 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a ternary nested in the first result of another ternary.
    #[test]
    fn test_reports_nested_then_ternary() {
        let session = TestSession::dir(
            &NO_NESTED_TERNARY,
            r#"
function select(first: boolean, second: boolean): int32 {
    return first ? (second ? 1 : 2) : 3;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-nested-ternary]: ternary is nested inside another ternary
 ──▶ main.tspp:2:20
  │
1 │ function select(first: boolean, second: boolean): int32 {
2 │     return first ? (second ? 1 : 2) : 3;
  │                    ^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept a ternary separated from another ternary by an operation.
    #[test]
    fn test_accepts_indirectly_nested_ternary() {
        let session = TestSession::dir(
            &NO_NESTED_TERNARY,
            r#"
function select(first: boolean, second: boolean): int32 {
    return first ? (second ? 1 : 2) + 1 : 3;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a ternary used directly as another ternary's condition.
    #[test]
    fn test_reports_nested_condition_ternary() {
        let session = TestSession::dir(
            &NO_NESTED_TERNARY,
            r#"
function select(first: boolean, second: boolean): int32 {
    return (first ? second : false) ? 1 : 2;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-nested-ternary]: ternary is nested inside another ternary
 ──▶ main.tspp:2:12
  │
1 │ function select(first: boolean, second: boolean): int32 {
2 │     return (first ? second : false) ? 1 : 2;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
