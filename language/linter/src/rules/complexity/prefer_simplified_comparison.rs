use crate::LintMeta;
use destack_dir::{self as dir, BinaryOperator};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_numeric_value, expression_unwrap_parenthesized_source_form};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest simplifying comparisons.
    ///
    /// Comparisons like `x >= y + 1` can be simplified to `x > y` for clarity.
    #[lint(
        id = "prefer-simplified-comparison",
        code = "LX026",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferSimplifiedComparison,
    "Suggest simplified comparisons"
}

impl LintRule for PreferSimplifiedComparison {
    fn meta(&self) -> &'static LintMeta {
        PreferSimplifiedComparison::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            let dir::Expression::Binary {
                operator,
                left,
                right,
            } = expression
            else {
                continue;
            };

            // check for known simplification forms
            let Some(simplification) = simplification_for_operator(ctx, *operator, *right) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build simplified replacement expression
            let expression_span = ctx.dir.get_span(node_id);
            let left_span = ctx.dir.get_span(*left);
            let right_span = ctx.dir.get_span(simplification.simplified_right);
            let left_text = ctx.get_span_text(left_span);
            let right_text = ctx.get_span_text(right_span);
            let replacement = format!("{left_text} {} {right_text}", simplification.operator_text);
            let edits = ctx
                .edit_builder()
                .replace(expression_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Simplify comparison by removing +/- 1").with_edits(edits);

            ctx.report(
                LintReport::new(
                    PREFER_SIMPLIFIED_COMPARISON.id,
                    PREFER_SIMPLIFIED_COMPARISON.code,
                    PREFER_SIMPLIFIED_COMPARISON.category,
                    severity,
                    simplification.message,
                    expression_span,
                )
                .label("simplify this comparison")
                .fix(fix),
            );
        }
    }
}

/// One simplification target for a comparison expression.
#[derive(Debug, Clone, Copy)]
struct ComparisonSimplification {
    /// The replacement operator text.
    operator_text: &'static str,
    /// The diagnostic message.
    message: &'static str,
    /// The expression to keep on the right side.
    simplified_right: dir::LocalNodeId<dir::Expression>,
}

/// Return one simplification for a comparison operator and right-hand side.
fn simplification_for_operator(
    ctx: &mut LintModuleContext<'_>,
    operator: BinaryOperator,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ComparisonSimplification> {
    match operator {
        BinaryOperator::GreaterThanOrEqual => {
            let base = right_add_or_sub_one(ctx, right_id, BinaryOperator::Add)?;
            Some(ComparisonSimplification {
                operator_text: ">",
                message: "can simplify `>= y + 1` to `> y`",
                simplified_right: base,
            })
        }
        BinaryOperator::LessThanOrEqual => {
            let base = right_add_or_sub_one(ctx, right_id, BinaryOperator::Subtract)?;
            Some(ComparisonSimplification {
                operator_text: "<",
                message: "can simplify `<= y - 1` to `< y`",
                simplified_right: base,
            })
        }
        BinaryOperator::GreaterThan => {
            let base = right_add_or_sub_one(ctx, right_id, BinaryOperator::Subtract)?;
            Some(ComparisonSimplification {
                operator_text: ">=",
                message: "can simplify `> y - 1` to `>= y`",
                simplified_right: base,
            })
        }
        BinaryOperator::LessThan => {
            let base = right_add_or_sub_one(ctx, right_id, BinaryOperator::Add)?;
            Some(ComparisonSimplification {
                operator_text: "<=",
                message: "can simplify `< y + 1` to `<= y`",
                simplified_right: base,
            })
        }
        _ => None,
    }
}

/// Return the base expression id when one side is `base +/- 1`.
fn right_add_or_sub_one(
    ctx: &mut LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    operator: BinaryOperator,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let expression = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let dir::Expression::Binary {
        left,
        operator: inner_operator,
        right,
    } = ctx.dir.get(expression)
    else {
        return None;
    };
    if *inner_operator != operator {
        return None;
    }

    let right_expression = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right);
    if expression_numeric_value(ctx, right_expression)? != 1.0 {
        return None;
    }

    Some(*left)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_gte_plus_one_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_gte_plus_one_detected.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x >= y + 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison")
            .assert_has_fix("prefer-simplified-comparison");
    }

    #[test]
    fn test_gt_comparison_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_gt_comparison_allowed.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x > y
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-simplified-comparison");
    }

    #[test]
    fn test_lte_minus_one_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_lte_minus_one_detected.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x <= y - 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison");
    }

    #[test]
    fn test_normal_comparison_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_normal_comparison_allowed.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x >= y
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-simplified-comparison");
    }

    #[test]
    fn test_fix_gte_plus_one() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_fix_gte_plus_one.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x >= y + 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison")
            .assert_safe_fixed(
                r#"
function foo(x: int32, y: int32): bool {
    return x > y;
}
"#,
            );
    }

    #[test]
    fn test_fix_lte_minus_one() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_fix_lte_minus_one.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x <= y - 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison")
            .assert_safe_fixed(
                r#"
function foo(x: int32, y: int32): bool {
    return x < y;
}
"#,
            );
    }

    #[test]
    fn test_fix_gt_minus_one() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_fix_gt_minus_one.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x > y - 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison")
            .assert_safe_fixed(
                r#"
function foo(x: int32, y: int32): bool {
    return x >= y;
}
"#,
            );
    }

    #[test]
    fn test_fix_lt_plus_one() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_fix_lt_plus_one.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x < y + 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison")
            .assert_safe_fixed(
                r#"
function foo(x: int32, y: int32): bool {
    return x <= y;
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_parenthesized_rhs_expression() {
        let test = TestProgram::for_rule_without_prelude(PreferSimplifiedComparison);
        let result = test.lint(
            "prefer_simplified_comparison/test_mutation_fix_parenthesized_rhs_expression.ds",
            r#"
function foo(x: int32, y: int32, z: int32): bool {
    return x >= (y + z) + 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison")
            .assert_safe_fixed(
                r#"
function foo(x: int32, y: int32, z: int32): bool {
    return x > (y + z);
}
"#,
            );
    }
}
