use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{expression_unwrap_parenthesized_source_form, is_comparison_operator};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow comparing to negative zero.
    ///
    /// Comparing to `-0` is almost always a mistake because `x === -0` is true for
    /// both `0` and `-0` in most languages. Use `Object.is(x, -0)` or equivalent
    /// to explicitly check for negative zero.
    #[lint(
        id = "no-compare-neg-zero",
        code = "LC007",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoCompareNegZero,
    "Disallow comparisons to negative zero"
}

impl LintRule for NoCompareNegZero {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoCompareNegZero::meta()
    }

    /// Check module source nodes for comparisons against negative zero.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // walk binary expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.dir.get(node_id)
            else {
                continue;
            };

            // only check comparison operators
            if !is_comparison_operator(operator) {
                continue;
            }

            // check if either side is negative zero
            let left_is_neg_zero = is_negative_zero(ctx, *left);
            let right_is_neg_zero = is_negative_zero(ctx, *right);

            // enforce this lint guard
            if !left_is_neg_zero && !right_is_neg_zero {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // resolve diagnostic span
            let expression_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_COMPARE_NEG_ZERO.id,
                NO_COMPARE_NEG_ZERO.code,
                NO_COMPARE_NEG_ZERO.category,
                severity,
                "comparison to negative zero",
                expression_span,
            )
            .label("use Object.is(x, -0) to check for negative zero");

            // construct fix for equality operators only
            if ctx.compute_fixes
                && let Some(fix) = make_neg_zero_fix(
                    ctx,
                    *left,
                    *right,
                    *operator,
                    left_is_neg_zero,
                    expression_span,
                )
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Create a fix for -0 comparison (equality operators only).
fn make_neg_zero_fix(
    ctx: &LintModuleContext<'_>,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
    left_is_neg_zero: bool,
    expression_span: destack_source::Span,
) -> Option<LintFix> {
    // get the non negative zero operand
    let other_id = if left_is_neg_zero { right } else { left };
    let other_span = ctx.dir.get_span(other_id);
    let other_text = ctx.get_span_text(other_span);

    // only strict equality operators are semantics preserving here
    let replacement = match operator {
        dir::BinaryOperator::EqualStrict => format!("Object.is({other_text}, -0)"),
        dir::BinaryOperator::NotEqualStrict => format!("!Object.is({other_text}, -0)"),

        // loose equality and relational operators: no fix
        _ => return None,
    };

    // build fix edits
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Replace with Object.is()").with_edits(edits))
}

/// Check if an expression is negative zero (-0).
fn is_negative_zero(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);

    // require unary negation of zero
    match expression {
        dir::Expression::Unary { operator, right } => {
            if *operator != dir::UnaryOperator::Negate {
                return false;
            }
            is_zero_literal(ctx, *right)
        }
        _ => false,
    }
}

/// Check if an expression is a zero literal (0 or 0.0).
fn is_zero_literal(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);

    // require numeric zero literals
    match expression {
        dir::Expression::ScalarLiteral(literal) => match literal {
            dir::ScalarLiteral::Integer(0) => true,
            dir::ScalarLiteral::Float(f) => *f == 0.0,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_compare_neg_zero_equality() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_compare_neg_zero_equality.ds",
            r#"
let x = 0;
x == -0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_compare_neg_zero_strict_equality() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_compare_neg_zero_strict_equality.ds",
            r#"
let x = 0;
x === -0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_compare_neg_zero_inequality() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_compare_neg_zero_inequality.ds",
            r#"
let x = 0;
x != -0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_does_not_fix_loose_equal() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_does_not_fix_loose_equal.ds",
            r#"
let x = 0;
if (x == -0) {
}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_has_no_fix("no-compare-neg-zero");
    }

    #[test]
    fn test_does_not_fix_loose_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_does_not_fix_loose_not_equal.ds",
            r#"
let x = 0;
if (x != -0) {
}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_has_no_fix("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_neg_zero_on_left() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_neg_zero_on_left.ds",
            r#"
let x = 0;
-0 === x;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_compare_neg_zero_float() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_compare_neg_zero_float.ds",
            r#"
let x = 0.0;
x === -0.0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_allows_compare_regular_zero() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_allows_compare_regular_zero.ds",
            r#"
let x = 0;
x === 0;
"#,
        );
        test.result(result).assert_no_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_allows_compare_positive_numbers() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_allows_compare_positive_numbers.ds",
            r#"
let x = 1;
x === 1;
"#,
        );
        test.result(result).assert_no_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_allows_negation_of_variable() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_allows_negation_of_variable.ds",
            r#"
let x = 1;
let y = 2;
x === -y;
"#,
        );
        test.result(result).assert_no_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_fix_strict_equal() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_fix_strict_equal.ds",
            r#"
let x = 0
if (x === -0) {}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_safe_fixed(
                r#"
let x = 0;
if (Object.is(x, -0)) {
}
"#,
            );
    }

    #[test]
    fn test_fix_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_fix_not_equal.ds",
            r#"
let x = 0
if (x !== -0) {}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_safe_fixed(
                r#"
let x = 0;
if (!Object.is(x, -0)) {
}
"#,
            );
    }

    #[test]
    fn test_fix_neg_zero_on_left() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_fix_neg_zero_on_left.ds",
            r#"
let x = 0
if (-0 === x) {}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_safe_fixed(
                r#"
let x = 0;
if (Object.is(x, -0)) {
}
"#,
            );
    }

    #[test]
    fn test_fix_complex_expression() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_fix_complex_expression.ds",
            r#"
let y = 0
let x = (y + 1) === -0
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_safe_fixed(
                r#"
let y = 0;
let x = Object.is((y + 1), -0);
"#,
            );
    }

    #[test]
    fn test_no_fix_for_relational() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_no_fix_for_relational.ds",
            r#"
let x = 0
if (x < -0) {}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_has_no_fix("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_parenthesized_negative_zero() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_parenthesized_negative_zero.ds",
            r#"
let x = 0;
if (x === (-0)) {
}
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_nested_parenthesized_negative_zero_on_right() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_nested_parenthesized_negative_zero_on_right.ds",
            r#"
let x = 0;
if ((x) !== (((-0)))) {
}
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_nested_parenthesized_negative_zero_on_left() {
        let test = TestProgram::for_rule_without_prelude(NoCompareNegZero);
        let result = test.lint(
            "no_compare_neg_zero/test_detects_nested_parenthesized_negative_zero_on_left.ds",
            r#"
let x = 0;
if (((-0)) >= (x)) {
}
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }
}
