use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow comparing to negative zero.
    ///
    /// Comparing to `-0` is almost always a mistake because `x === -0` is true for
    /// both `0` and `-0` in most languages. Use `Object.is(x, -0)` or equivalent
    /// to explicitly check for negative zero.
    #[lint(
        id = "no-compare-neg-zero",
        code = "LC005",
        category = Correctness,
        level = Ast,
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoCompareNegZero,
    "Disallow comparisons to negative zero"
}

impl LintRule for NoCompareNegZero {
    fn meta(&self) -> &'static crate::LintMeta {
        NoCompareNegZero::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // only check comparison operators
            if !is_comparison_operator(*operator) {
                continue;
            }

            // check if either side is -0
            let left_is_neg_zero = is_negative_zero(ctx, *left);
            let right_is_neg_zero = is_negative_zero(ctx, *right);

            if !left_is_neg_zero && !right_is_neg_zero {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_COMPARE_NEG_ZERO.id,
                NO_COMPARE_NEG_ZERO.code,
                NO_COMPARE_NEG_ZERO.category,
                severity,
                "comparison to negative zero",
                ctx.module.file_id,
                expression_span,
            )
            .with_label("use Object.is(x, -0) to check for negative zero");

            // construct fix for equality operators only
            if let Some(fix) = make_neg_zero_fix(
                ctx,
                *left,
                *right,
                *operator,
                left_is_neg_zero,
                expression_span,
            ) {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

fn is_comparison_operator(operator: ast::BinaryOperator) -> bool {
    matches!(
        operator,
        ast::BinaryOperator::Equal
            | ast::BinaryOperator::NotEqual
            | ast::BinaryOperator::EqualStrict
            | ast::BinaryOperator::NotEqualStrict
            | ast::BinaryOperator::LessThan
            | ast::BinaryOperator::LessThanOrEqual
            | ast::BinaryOperator::GreaterThan
            | ast::BinaryOperator::GreaterThanOrEqual
    )
}

/// Create a fix for -0 comparison (equality operators only).
fn make_neg_zero_fix(
    ctx: &LintModuleAstContext<'_>,
    left: ast::LocalNodeId<ast::Expression>,
    right: ast::LocalNodeId<ast::Expression>,
    operator: ast::BinaryOperator,
    left_is_neg_zero: bool,
    expression_span: destack_source::Span,
) -> Option<LintFix> {
    // get the non-(-0) operand
    let other_id = if left_is_neg_zero { right } else { left };
    let other_span = ctx.tree.get_span(other_id);
    let other_text = ctx.get_span_text(other_span);

    // only fix equality operators - relational comparisons don't have a meaningful fix
    let replacement = match operator {
        ast::BinaryOperator::Equal | ast::BinaryOperator::EqualStrict => {
            format!("Object.is({other_text}, -0)")
        }
        ast::BinaryOperator::NotEqual | ast::BinaryOperator::NotEqualStrict => {
            format!("!Object.is({other_text}, -0)")
        }
        // relational operators - no fix
        _ => return None,
    };

    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Replace with Object.is()").with_edits(edits))
}

/// Check if an expression is negative zero (-0).
fn is_negative_zero(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        // -0 is a unary negation of a zero literal
        ast::Expression::Unary { operator, right } => {
            if *operator != ast::UnaryOperator::Negate {
                return false;
            }
            is_zero_literal(ctx, *right)
        }
        ast::Expression::Parenthesized { expression } => is_negative_zero(ctx, *expression),
        _ => false,
    }
}

/// Check if an expression is a zero literal (0 or 0.0).
fn is_zero_literal(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::ScalarLiteral(literal) => match literal {
            ast::ScalarLiteral::Integer(0) => true,
            ast::ScalarLiteral::Float(f) => *f == 0.0,
            _ => false,
        },
        ast::Expression::Parenthesized { expression } => is_zero_literal(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_compare_neg_zero_equality() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0;
x == -0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_compare_neg_zero_strict_equality() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0;
x === -0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_compare_neg_zero_inequality() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0;
x != -0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_neg_zero_on_left() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0;
-0 === x;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_detects_compare_neg_zero_float() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0.0;
x === -0.0;
"#,
        );
        test.result(result).assert_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_allows_compare_regular_zero() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0;
x === 0;
"#,
        );
        test.result(result).assert_no_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_allows_compare_positive_numbers() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
x === 1;
"#,
        );
        test.result(result).assert_no_lint("no-compare-neg-zero");
    }

    #[test]
    fn test_allows_negation_of_variable() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
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
if (Object.is(x, -0)) { }
"#,
            );
    }

    #[test]
    fn test_fix_not_equal() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
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
if (!Object.is(x, -0)) { }
"#,
            );
    }

    #[test]
    fn test_fix_neg_zero_on_left() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
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
if (Object.is(x, -0)) { }
"#,
            );
    }

    #[test]
    fn test_fix_complex_expression() {
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoCompareNegZero);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 0
if (x < -0) {}
"#,
        );
        test.result(result)
            .assert_lint("no-compare-neg-zero")
            .assert_has_no_fix("no-compare-neg-zero");
    }
}
