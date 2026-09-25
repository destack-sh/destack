use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow comparisons with negative zero.
    pub NO_COMPARE_NEG_ZERO {
        id: "no-compare-neg-zero",
        summary: "Disallow comparisons with negative zero",
        explanation: r#"
Equality and ordering comparisons produce the same result for negative and positive zero, so a comparison against negative zero cannot test its sign.
Instead, you SHOULD compare with zero and call `.isSignNegative()` when the sign matters.
"#,
        example: {
            reported: r#"
function isNegativeZero(value: float64): boolean {
    return value === -0.0;
}
"#,
            accepted: r#"
function isNegativeZero(value: float64): boolean {
    return value === 0.0 && value.isSignNegative();
}
"#,
        },
        provenance: [Eslint("no-compare-neg-zero")],
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report comparisons whose authored operand is negative floating-point zero.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect comparison expressions
    for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = expression
        else {
            continue;
        };
        if !operator.is_comparison() {
            continue;
        }
        let Some(resolution) = module.operator_decision(expression_id.into_any())? else {
            continue;
        };
        if !resolution.is_builtin() {
            continue;
        }

        // select one negative-zero operand
        let negative_zero = if module.is_negative_zero(*left)? {
            *left
        } else if module.is_negative_zero(*right)? {
            *right
        } else {
            continue;
        };

        // report the indistinguishable comparison operand
        let span = module.source_extent(negative_zero.into_any())?;
        let diagnostic = lint
            .diagnostic("comparison cannot distinguish negative zero", span)
            .help("compare with zero and call `.isSignNegative()` to inspect the sign bit");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report negative zero on the left of an ordering comparison.
    #[test]
    fn test_reports_reversed_negative_zero_comparison() {
        let session = TestSession::dir(
            &NO_COMPARE_NEG_ZERO,
            r#"
function isPositive(value: float64): boolean {
    return -0.0 < value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-compare-neg-zero]: comparison cannot distinguish negative zero
 ──▶ main.tspp:2:12
  │
1 │ function isPositive(value: float64): boolean {
2 │     return -0.0 < value;
  │            ^^^^
3 │ }
  │

 = help: compare with zero and call `.isSignNegative()` to inspect the sign bit
"#,
        );
    }

    /// Accept positive floating-point zero.
    #[test]
    fn test_accepts_positive_zero_comparison() {
        let session = TestSession::dir(
            &NO_COMPARE_NEG_ZERO,
            r#"
function isZero(value: float64): boolean {
    return value === 0.0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept negated integer zero, which has no sign bit.
    #[test]
    fn test_accepts_integer_zero_comparison() {
        let session = TestSession::dir(
            &NO_COMPARE_NEG_ZERO,
            r#"
function isZero(value: int32): boolean {
    return value === -0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept negative zero passed to user-defined equality.
    #[test]
    fn test_accepts_overloaded_negative_zero_comparison() {
        let session = TestSession::dir(
            &NO_COMPARE_NEG_ZERO,
            r#"
import { PartialEqual } from "tspp:ops";

struct Measure {
    value: float64;
}

extension of Measure implements PartialEqual<float64> {
    equal(&readonly this, other: &readonly float64): boolean {
        return this.value == other;
    }
}

declare const measure: Measure;
const same = measure == -0.0;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a named negative-zero constant.
    #[test]
    fn test_reports_named_negative_zero_comparison() {
        let session = TestSession::dir(
            &NO_COMPARE_NEG_ZERO,
            r#"
const NegativeZero = -0.0;
function isNegativeZero(value: float64): boolean {
    return value === NegativeZero;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-compare-neg-zero]: comparison cannot distinguish negative zero
 ──▶ main.tspp:3:22
  │
1 │ const NegativeZero = -0.0;
2 │ function isNegativeZero(value: float64): boolean {
3 │     return value === NegativeZero;
  │                      ^^^^^^^^^^^^
4 │ }
  │

 = help: compare with zero and call `.isSignNegative()` to inspect the sign bit
"#,
        );
    }
}
