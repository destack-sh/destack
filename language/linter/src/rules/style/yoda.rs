use crate::LintMeta;
use destack_dir::{self as dir, BinaryOperator, Expression, UnaryOperator};
use destack_repository::{LintSeverity, YodaMode};

use crate::rules::common::{
    expression_is_equal, expression_is_literal, expression_outer_parenthesized_source_form,
    expression_static_string_literal_source_form, expression_unwrap_parenthesized_source_form,
    is_comparison_operator, source_text_contains_comment_token,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow "Yoda" conditions.
    ///
    /// Yoda conditions are comparisons where the literal value comes first,
    /// like `"red" === color` instead of `color === "red"`. While syntactically
    /// valid, they can be confusing and are less natural to read.
    #[lint(
        id = "yoda",
        code = "LY066",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub Yoda,
    "Disallow Yoda conditions"
}

impl LintRule for Yoda {
    fn meta(&self) -> &'static LintMeta {
        Yoda::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mode = ctx.options().style.yoda_mode;

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };

            // only check comparison operators
            if !is_comparison_operator(operator) {
                continue;
            }

            // honor the equality-only option
            if ctx.options().style.yoda_only_equality && !is_equality_operator(operator) {
                continue;
            }

            // honor the range exception option
            if ctx.options().style.yoda_except_range
                && expression_is_part_of_range_test(ctx, node_id)
            {
                continue;
            }

            // check literal placement against the configured mode
            let left_expression = ctx.dir.get(*left);
            let right_expression = ctx.dir.get(*right);
            let left_is_literal = expression_looks_like_literal(ctx, *left, left_expression);
            let right_is_literal = expression_looks_like_literal(ctx, *right, right_expression);
            let expected_literal_on_left = mode == YodaMode::Always;
            let needs_report = if expected_literal_on_left {
                right_is_literal && !left_is_literal
            } else {
                left_is_literal && !right_is_literal
            };
            if !needs_report {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // make fix: flip comparison
            let expression_span = ctx.dir.get_span(node_id);
            let left_span = ctx.dir.get_span(*left);
            let right_span = ctx.dir.get_span(*right);
            let left_text = ctx.get_span_text(left_span);
            let right_text = ctx.get_span_text(right_span);
            let expected_side = if expected_literal_on_left {
                "left"
            } else {
                "right"
            };

            let mut diagnostic = LintReport::new(
                YODA.id,
                YODA.code,
                YODA.category,
                severity,
                format!("expected literal to be on the {expected_side} side of comparison"),
                expression_span,
            )
            .label(format!("move the literal to the {expected_side} side"));

            if !source_text_contains_comment_token(ctx.get_span_text(expression_span)) {
                let flipped_op = flip_operator(operator);
                let replacement = format!("{right_text} {flipped_op} {left_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Flip comparison").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when the comparison operator is equality based.
fn is_equality_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
    )
}

/// Flip a comparison operator for yoda fix (e.g., < becomes >).
fn flip_operator(operator: &BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::EqualStrict => "===",
        BinaryOperator::NotEqualStrict => "!==",
        BinaryOperator::LessThan => ">",
        BinaryOperator::LessThanOrEqual => ">=",
        BinaryOperator::GreaterThan => "<",
        BinaryOperator::GreaterThanOrEqual => "<=",
        _ => unreachable!("only called for comparison operators"),
    }
}

/// Return true when one expression should be treated like a literal for yoda checks.
fn expression_looks_like_literal(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &Expression,
) -> bool {
    if expression_is_literal(expression) {
        return true;
    }

    if expression_static_string_literal_source_form(ctx.dir.tree(), expression_id).is_some() {
        return true;
    }

    matches!(
        expression,
        Expression::Unary {
            operator: UnaryOperator::Negate,
            right,
        } if matches!(
            ctx.dir.get(*right),
            Expression::ScalarLiteral(dir::ScalarLiteral::Integer(_))
                | Expression::ScalarLiteral(dir::ScalarLiteral::Float(_))
                | Expression::ScalarLiteral(dir::ScalarLiteral::Bigint(_))
        )
    )
}

/// Return true when a comparison participates in a range-test logical expression.
fn expression_is_part_of_range_test(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_outer_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let Some(parent_id) = ctx.dir.get_parent_id(expression_id.id) else {
        return false;
    };
    if ctx.dir.get_node_type(parent_id) != dir::NodeType::Expression {
        return false;
    }

    let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = ctx.dir.get(parent_expression_id);
    let Expression::Binary {
        left,
        operator,
        right,
    } = parent_expression
    else {
        return false;
    };
    if !matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
        return false;
    }

    let left_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left);
    let right_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right);
    let left_expression = ctx.dir.get(left_id);
    let right_expression = ctx.dir.get(right_id);
    let (
        Expression::Binary {
            left: left_left,
            operator: left_operator,
            right: left_right,
        },
        Expression::Binary {
            left: right_left,
            operator: right_operator,
            right: right_right,
        },
    ) = (left_expression, right_expression)
    else {
        return false;
    };
    if !is_range_operator(left_operator) || !is_range_operator(right_operator) {
        return false;
    }

    let left_left = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left_left);
    let left_right = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left_right);
    let right_left = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right_left);
    let right_right = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right_right);

    let is_between_range = expression_is_equal(ctx, left_right, right_left)
        && expression_looks_like_literal(ctx, left_left, ctx.dir.get(left_left))
        && expression_looks_like_literal(ctx, right_right, ctx.dir.get(right_right));
    let is_outside_range = expression_is_equal(ctx, left_left, right_right)
        && expression_looks_like_literal(ctx, left_right, ctx.dir.get(left_right))
        && expression_looks_like_literal(ctx, right_left, ctx.dir.get(right_left));

    is_between_range || is_outside_range
}

/// Return true when the operator can participate in a range test.
fn is_range_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_yoda_equality() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_detects_yoda_equality.ds",
            r#"
if ("red" === color) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_strict_equality() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_detects_yoda_strict_equality.ds",
            r#"
if (5 === x) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_less_than() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_detects_yoda_less_than.ds",
            r#"
if (10 < x) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_null_check() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_detects_yoda_null_check.ds",
            r#"
if (null === value) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_allows_normal_comparison() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_allows_normal_comparison.ds",
            r#"
if (color === "red") {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_variable_comparison() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_allows_variable_comparison.ds",
            r#"
if (a === b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_literal_to_literal() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_allows_literal_to_literal.ds",
            r#"
if (5 === 5) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_non_comparison_operators() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_allows_non_comparison_operators.ds",
            r#"
let x = 5 + a;
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_fix_yoda_equality() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_fix_yoda_equality.ds",
            r#"
if (5 === x) { y() }
"#,
        );
        test.result(result).assert_lint("yoda").assert_safe_fixed(
            r#"
if (x === 5) { y() }
"#,
        );
    }

    #[test]
    fn test_has_no_fix_when_comparison_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_has_no_fix_when_comparison_contains_comment.ds",
            r#"
if (1 /* keep */ === value) {
    ok()
}
"#,
        );
        test.result(result)
            .assert_lint("yoda")
            .assert_has_no_fix("yoda");
    }

    #[test]
    fn test_fix_yoda_less_than() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_fix_yoda_less_than.ds",
            r#"
if (10 < x) { y() }
"#,
        );
        test.result(result).assert_lint("yoda").assert_safe_fixed(
            r#"
if (x > 10) { y() }
"#,
        );
    }

    #[test]
    fn test_fix_yoda_greater_than_or_equal() {
        let test = TestProgram::for_rule_without_prelude(Yoda);
        let result = test.lint(
            "yoda/test_fix_yoda_greater_than_or_equal.ds",
            r#"
if (10 >= x) { y() }
"#,
        );
        test.result(result).assert_lint("yoda").assert_safe_fixed(
            r#"
if (x <= 10) { y() }
"#,
        );
    }

    #[test]
    fn test_allows_non_yoda_in_always_mode() {
        let test = TestProgram::for_rule_without_prelude(Yoda)
            .with_options(|options| options.style.yoda_mode = YodaMode::Always);
        let result = test.lint(
            "yoda/test_allows_non_yoda_in_always_mode.ds",
            r#"
if (value === 5) { y() }
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_allows_range_test_when_except_range_enabled() {
        let test = TestProgram::for_rule_without_prelude(Yoda).with_options(|options| {
            options.style.yoda_except_range = true;
        });
        let result = test.lint(
            "yoda/test_allows_range_test_when_except_range_enabled.ds",
            r#"
if ((0 <= x) && (x < 10)) { y() }
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_non_equality_when_only_equality_enabled() {
        let test = TestProgram::for_rule_without_prelude(Yoda).with_options(|options| {
            options.style.yoda_only_equality = true;
        });
        let result = test.lint(
            "yoda/test_allows_non_equality_when_only_equality_enabled.ds",
            r#"
if (10 < x) { y() }
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }
}
