use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require an absolute difference in `Number.EPSILON` comparisons.
    pub FLOAT_EQUALITY_WITHOUT_ABS {
        id: "float-equality-without-abs",
        summary: "Require an absolute difference in `Number.EPSILON` comparisons",
        explanation: r#"
`left - right < Number.EPSILON` accepts every negative difference, including values that are arbitrarily far apart.
Instead, you SHOULD compare the absolute difference: `(left - right).abs() < Number.EPSILON`.

The absolute value is unnecessary only when `left >= right` is a proven invariant.
"#,
        example: {
            reported: r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return left - right < Number.EPSILON;
}
"#,
            accepted: r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return (left - right).abs() < Number.EPSILON;
}
"#,
        },
        provenance: [Clippy("float_equality_without_abs")],
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report one-sided floating-point differences compared with the canonical epsilon.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect compiler-defined ordering comparisons
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };

        // orient the comparison with the possible difference on the left
        let (difference, epsilon) = match operator {
            dir::BinaryOperator::LessThan | dir::BinaryOperator::LessThanOrEqual => {
                (left.source.local_id, right.source.local_id)
            }
            dir::BinaryOperator::GreaterThan | dir::BinaryOperator::GreaterThanOrEqual => {
                (right.source.local_id, left.source.local_id)
            }
            _ => continue,
        };
        let number_epsilon = dir::LanguageItem::Number.member("EPSILON");
        if module.language_member(epsilon)? != Some(number_epsilon) {
            continue;
        }

        // require builtin subtraction between floating-point operands
        if !matches!(view.get(difference), dir::Expression::Binary { .. }) {
            continue;
        }
        let Some((dir::BinaryOperator::Subtract, [left_operand, right_operand])) =
            module.builtin_binary(difference)?
        else {
            continue;
        };
        let float = dir::ScalarFamily::Domain(dir::ScalarDomain::Float);
        let is_float =
            left_operand.has_scalar_family(float) && right_operand.has_scalar_family(float);
        if !is_float {
            continue;
        }

        // report the one-sided difference and append the missing operation
        let span = module.source_extent(difference.into_any())?;
        let diagnostic = lint
            .diagnostic(
                "floating-point difference is compared without an absolute value",
                span,
            )
            .suggestion(suggestion(module, lint, difference)?);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Append an absolute-value call without discarding source text.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    difference: dir::LocalNodeId<dir::Expression>,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let extent = module.source_extent(difference.into_any())?;

    // retain existing parentheses and their comments when present
    let (extent, replacement) =
        if let Some(parentheses) = module.source_parentheses(difference.into_any()) {
            let source = module.source(parentheses)?;

            (parentheses, format!("{source}.abs()"))
        } else {
            let source = module.expression_source(difference, dir::OperatorPrecedence::Postfix)?;

            (extent, format!("{source}.abs()"))
        };

    // append the missing operation to the complete difference
    let patch = Patch::replace(extent, replacement);

    lint.suggestion("take the absolute difference", patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Suggest taking the absolute value of a floating-point difference.
    #[test]
    fn test_reports_difference_without_absolute_value() {
        let session = TestSession::dir(
            &FLOAT_EQUALITY_WITHOUT_ABS,
            r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return left - right < Number.EPSILON;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[float-equality-without-abs]: floating-point difference is compared without an absolute value
 ──▶ main.tspp:2:12
  │
1 │ function approximatelyEqual(left: float64, right: float64): boolean {
2 │     return left - right < Number.EPSILON;
  │            ^^^^^^^^^^^^
3 │ }
  │

 = suggestion: take the absolute difference (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function approximatelyEqual(left: float64, right: float64): boolean {
-   2│     return left - right < Number.EPSILON;
+   2│     return (left - right).abs() < Number.EPSILON;
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return (left - right).abs() < Number.EPSILON;
}
"#,
        );
    }

    /// Recognize the equivalent comparison with reversed operands.
    #[test]
    fn test_reports_reversed_epsilon_comparison() {
        let session = TestSession::dir(
            &FLOAT_EQUALITY_WITHOUT_ABS,
            r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return Number.EPSILON > left - right;
}
"#,
        );

        session.assert_suggestions(
            r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return Number.EPSILON > (left - right).abs();
}
"#,
        );
    }

    /// Accept an absolute difference and a domain-specific tolerance.
    #[test]
    fn test_accepts_other_tolerance_comparisons() {
        let session = TestSession::dir(
            &FLOAT_EQUALITY_WITHOUT_ABS,
            r#"
function approximatelyEqual(left: float64, right: float64): boolean {
    return (left - right).abs() < Number.EPSILON || left - right < 0.001;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
