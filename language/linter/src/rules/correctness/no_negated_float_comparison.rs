use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow negated floating-point ordering comparisons that accept NaN.
    pub NO_NEGATED_FLOAT_COMPARISON {
        id: "no-negated-float-comparison",
        summary: "Disallow negated floating-point ordering comparisons that accept NaN",
        explanation: r#"
`!(left < right)` evaluates to `true` when either operand is NaN because every ordering comparison with NaN evaluates to `false`.
Instead, you SHOULD handle NaN explicitly and write the intended positive comparison, such as `!left.isNaN() && !right.isNaN() && left >= right`.
"#,
        example: {
            reported: r#"
function isAtLeast(left: float64, right: float64): boolean {
    return !(left < right);
}
"#,
            accepted: r#"
function isAtLeast(left: float64, right: float64): boolean {
    return !left.isNaN() && !right.isNaN() && left >= right;
}
"#,
        },
        provenance: [Clippy("neg_cmp_op_on_partial_ord")],
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report negated builtin floating-point ordering comparisons.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined boolean negations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((dir::UnaryOperator::Not, operand)) = module.builtin_unary(expression)? else {
            continue;
        };
        let comparison = operand.source.local_id;
        if !matches!(
            module.view().get(comparison),
            dir::Expression::Binary { .. }
        ) {
            continue;
        };

        // require one builtin float ordering comparison beneath the negation
        let Some((operator, [left_operand, right_operand])) = module.builtin_binary(comparison)?
        else {
            continue;
        };
        let is_ordering = matches!(
            operator,
            dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        );
        let float = dir::ScalarFamily::Domain(dir::ScalarDomain::Float);
        let has_float =
            left_operand.has_scalar_family(float) || right_operand.has_scalar_family(float);
        if !is_ordering || !has_float {
            continue;
        }

        // report the complete negated comparison
        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("negated floating-point comparison accepts NaN", span)
            .help("handle NaN explicitly and use the intended positive comparison");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report negation of a floating-point ordering comparison.
    #[test]
    fn test_reports_negated_float_comparison() {
        let session = TestSession::dir(
            &NO_NEGATED_FLOAT_COMPARISON,
            r#"
function isAtLeast(left: float64, right: float64): boolean {
    return !(left < right);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-negated-float-comparison]: negated floating-point comparison accepts NaN
 ──▶ main.tspp:2:12
  │
1 │ function isAtLeast(left: float64, right: float64): boolean {
2 │     return !(left < right);
  │            ^^^^^^^^^^^^^^
3 │ }
  │

 = help: handle NaN explicitly and use the intended positive comparison
"#,
        );
    }

    /// Accept negated integer ordering and floating-point equality.
    #[test]
    fn test_accepts_other_negated_comparisons() {
        let session = TestSession::dir(
            &NO_NEGATED_FLOAT_COMPARISON,
            r#"
function integer(left: int32, right: int32): boolean {
    return !(left < right);
}
function floating(left: float64, right: float64): boolean {
    return !(left === right);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicit positive float ordering.
    #[test]
    fn test_accepts_positive_float_comparison() {
        let session = TestSession::dir(
            &NO_NEGATED_FLOAT_COMPARISON,
            r#"
function isAtLeast(left: float64, right: float64): boolean {
    return left >= right;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
