use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow exact equality comparisons between floating-point values.
    pub NO_FLOATING_POINT_EQUALITY {
        id: "no-floating-point-equality",
        summary: "Disallow exact equality comparisons between floating-point values",
        explanation: r#"
Floating-point arithmetic rounds intermediate results, so mathematically equivalent calculations can produce different representations.
Instead, you SHOULD compare an absolute or relative error with a tolerance chosen for the value's scale and domain.
"#,
        example: {
            reported: r#"
function same(left: float64, right: float64): boolean {
    return left === right;
}
"#,
            accepted: r#"
function same(left: float64, right: float64): boolean {
    return (left - right).abs() < 0.001;
}
"#,
        },
        provenance: [Clippy("float_cmp")],
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report exact builtin equality between floating-point values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined equality operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, [left_operand, right_operand])) = module.builtin_binary(expression)?
        else {
            continue;
        };
        let float = dir::ScalarFamily::Domain(dir::ScalarDomain::Float);
        let has_float =
            left_operand.has_scalar_family(float) || right_operand.has_scalar_family(float);
        if !operator.is_equality() || !has_float {
            continue;
        }

        // retain exact zero and infinity comparisons
        let left = left_operand.source.local_id;
        let right = right_operand.source.local_id;
        let left_constant = module.scalar_constant(left)?;
        let right_constant = module.scalar_constant(right)?;
        let has_zero = matches!(left_constant, Some(dir::Literal::Float(value)) if value == 0.0)
            || matches!(right_constant, Some(dir::Literal::Float(value)) if value == 0.0);
        let has_infinity = module.is_infinite(left)? || module.is_infinite(right)?;
        if has_zero || has_infinity {
            continue;
        }

        // retain NaN checks, self-comparisons, and equality implementations
        let has_nan = module.is_nan(left)? || module.is_nan(right)?;
        let is_same_operand = module.is_same_operand(left_operand, right_operand)?;
        let equality = dir::LanguageItem::PartialEqual.member("equal");
        let defines_equality = module.is_within_language_member(expression.into_any(), equality)?;
        if has_nan || is_same_operand || defines_equality {
            continue;
        }

        // report the complete exact comparison
        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("floating-point values are compared exactly", span)
            .help("compare an absolute or relative error with a domain-specific tolerance");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report exact equality between computed floats.
    #[test]
    fn test_reports_float_equality() {
        let session = TestSession::dir(
            &NO_FLOATING_POINT_EQUALITY,
            r#"
function same(left: float64, right: float64): boolean {
    return left === right;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-floating-point-equality]: floating-point values are compared exactly
 ──▶ main.tspp:2:12
  │
1 │ function same(left: float64, right: float64): boolean {
2 │     return left === right;
  │            ^^^^^^^^^^^^^^
3 │ }
  │

 = help: compare an absolute or relative error with a domain-specific tolerance
"#,
        );
    }

    /// Accept zero, infinity, NaN, and repeated operand comparisons.
    #[test]
    fn test_accepts_float_sentinels() {
        let session = TestSession::dir(
            &NO_FLOATING_POINT_EQUALITY,
            r#"
function classify(value: float64): boolean {
    return value === 0.0 ||
    value === Infinity ||
    value === Number.NEGATIVE_INFINITY ||
    value !== -Number.POSITIVE_INFINITY ||
    value !== NaN ||
    value !== value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept exact comparison selected through a user-defined protocol.
    #[test]
    fn test_accepts_overloaded_equality() {
        let session = TestSession::dir(
            &NO_FLOATING_POINT_EQUALITY,
            r#"
import { PartialEqual } from "tspp:ops";

struct Measure {
    value: float64;
}

extension of Measure implements PartialEqual<Measure> {
    equal(&readonly this, other: &readonly Measure): boolean {
        return this.value === other.value;
    }
}

declare const left: Measure;
declare const right: Measure;
const same = left == right;
"#,
        );

        session.assert_no_diagnostics();
    }
}
