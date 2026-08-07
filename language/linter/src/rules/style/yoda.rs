use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow literal-first comparisons.
    pub YODA {
        id: "yoda",
        summary: "Disallow literal-first comparisons",
        explanation: "Putting a literal before the value being tested reverses the usual subject-first reading order. Put the checked value first and reverse relational operators so the comparison retains its meaning.",
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
    for expression in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = view.get(expression)
        else {
            continue;
        };
        if !operator.is_comparison()
            || view.get(*left).as_scalar().is_none()
            || view.get(*right).as_scalar().is_some()
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
        if let Some(suggestion) = suggestion(module, lint, expression, *left, *right, operator)? {
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
    let mut file = FilePatch::new(extent.file);
    file.replace(extent, replacement);
    let patches = PatchSet::single(file);
    let suggestion = lint.fix("put the checked value first", patches)?;

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
 ──▶ main.ds:2:12
  │
1 │ function isLarge(value: int32): boolean {
2 │     return 10 < value;
  │            ^^^^^^^^^^
3 │ }
  │

 = fix: put the checked value first
--- a/main.ds
+++ b/main.ds

    1│ function isLarge(value: int32): boolean {
-   2│     return 10 < value;
+   2│     return value > 10;
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
