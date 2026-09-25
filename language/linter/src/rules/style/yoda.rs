use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow literal-first comparisons.
    pub YODA {
        id: "yoda",
        summary: "Disallow literal-first comparisons",
        explanation: r#"
A literal-first comparison reverses the subject-first order used by other comparisons.
Instead, you SHOULD put the checked value first and reverse a relational operator when required.
"#,
        example: {
            reported: r#"
function isReady(state: string): boolean {
    return "ready" === state;
}
"#,
            accepted: r#"
function isReady(state: string): boolean {
    return state === "ready";
}
"#,
        },
        provenance: [Eslint("yoda")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report builtin comparisons with a literal before a nonliteral value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin comparisons in literal-first order
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        let left = left.source.local_id;
        let right = right.source.local_id;

        // require a scalar literal only on the left
        if !operator.is_comparison()
            || view.get(left).as_scalar().is_none()
            || view.get(right).as_scalar().is_some()
        {
            continue;
        }
        let Some(resolution) = module.operator_decision(expression.into_any())? else {
            continue;
        };
        if !resolution.is_builtin() {
            continue;
        }
        let Some(operator) = operator.swapped() else {
            continue;
        };

        // report and reverse the exact authored operands
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("comparison puts its literal first", span);
        if let Some(suggestion) = suggestion(module, lint, expression, left, right, operator)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the exact subject-first comparison replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let left = module.source_extent(left.into_any())?;
    let right = module.source_extent(right.into_any())?;

    // do not move comments detached from either operand
    if module.has_unretained_comment(extent, &[left, right])? {
        return Ok(None);
    }

    // swap the operands and reverse relational direction
    let replacement = format!(
        "{} {} {}",
        module.source(right)?,
        operator.text(),
        module.source(left)?
    );
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("put the checked value first", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Reverse a literal-first relational comparison.
    #[test]
    fn test_reverses_relational_comparison() {
        let session = TestSession::dir(
            &YODA,
            r#"
function isLarge(value: int32): boolean {
    return 10 < value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[yoda]: comparison puts its literal first
 ──▶ main.tspp:2:12
  │
1 │ function isLarge(value: int32): boolean {
2 │     return 10 < value;
  │            ^^^^^^^^^^
3 │ }
  │

 = fix: put the checked value first
--- a/main.tspp
+++ b/main.tspp

    1│ function isLarge(value: int32): boolean {
-   2│     return 10 < value;
+   2│     return value > 10;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function isLarge(value: int32): boolean {
    return value > 10;
}
"#,
        );
    }

    /// Accept the ordinary subject-first comparison order.
    #[test]
    fn test_accepts_subject_first_comparison() {
        let session = TestSession::dir(
            &YODA,
            r#"
function isLarge(value: int32): boolean {
    return value > 10;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept comparisons between two authored literals.
    #[test]
    fn test_accepts_two_literal_operands() {
        let session = TestSession::dir(
            &YODA,
            r#"
const ordered = 1 < 2;
"#,
        );

        session.assert_no_diagnostics();
    }
}
