use destack_ast::{self as ast, BinaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest simplifying comparisons.
    ///
    /// Comparisons like `x >= y + 1` can be simplified to `x > y` for clarity.
    #[lint(
        id = "prefer-simplified-comparison",
        code = "LX015",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferSimplifiedComparison,
    "Suggest simplified comparisons"
}

impl LintRule for PreferSimplifiedComparison {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferSimplifiedComparison::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::Binary {
                operator,
                left: _,
                right,
            } = expression
            else {
                continue;
            };

            // check for >= or <= comparisons
            let (simplify_msg, can_simplify) = match operator {
                BinaryOperator::GreaterThanOrEqual => {
                    // x >= y + 1 can be x > y
                    let right_expr = ctx.tree.get(*right);
                    if is_add_one(ctx, right_expr) {
                        (Some("can simplify `>= y + 1` to `> y`"), true)
                    } else {
                        (None, false)
                    }
                }
                BinaryOperator::LessThanOrEqual => {
                    // x <= y - 1 can be x < y
                    let right_expr = ctx.tree.get(*right);
                    if is_sub_one(ctx, right_expr) {
                        (Some("can simplify `<= y - 1` to `< y`"), true)
                    } else {
                        (None, false)
                    }
                }
                BinaryOperator::GreaterThan => {
                    // x > y - 1 can be x >= y
                    let right_expr = ctx.tree.get(*right);
                    if is_sub_one(ctx, right_expr) {
                        (Some("can simplify `> y - 1` to `>= y`"), true)
                    } else {
                        (None, false)
                    }
                }
                BinaryOperator::LessThan => {
                    // x < y + 1 can be x <= y
                    let right_expr = ctx.tree.get(*right);
                    if is_add_one(ctx, right_expr) {
                        (Some("can simplify `< y + 1` to `<= y`"), true)
                    } else {
                        (None, false)
                    }
                }
                _ => (None, false),
            };

            if can_simplify {
                ctx.report(
                    LintDiagnostic::new(
                        PREFER_SIMPLIFIED_COMPARISON.id,
                        PREFER_SIMPLIFIED_COMPARISON.code,
                        PREFER_SIMPLIFIED_COMPARISON.category,
                        severity,
                        simplify_msg.unwrap_or("comparison can be simplified"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("simplify this comparison"),
                );
            }
        }
    }
}

fn is_add_one(ctx: &LintModuleAstContext<'_>, expr: &ast::Expression) -> bool {
    if let ast::Expression::Binary {
        operator: BinaryOperator::Add,
        right,
        ..
    } = expr
    {
        let right_expr = ctx.tree.get(*right);
        return is_literal_one(right_expr);
    }
    false
}

fn is_sub_one(ctx: &LintModuleAstContext<'_>, expr: &ast::Expression) -> bool {
    if let ast::Expression::Binary {
        operator: BinaryOperator::Subtract,
        right,
        ..
    } = expr
    {
        let right_expr = ctx.tree.get(*right);
        return is_literal_one(right_expr);
    }
    false
}

fn is_literal_one(expr: &ast::Expression) -> bool {
    if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::Integer(value)) = expr {
        return *value == 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_gte_plus_one_detected() {
        let test = TestProgram::for_rule(PreferSimplifiedComparison);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x >= y + 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-simplified-comparison");
    }

    #[test]
    fn test_gt_comparison_allowed() {
        let test = TestProgram::for_rule(PreferSimplifiedComparison);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(PreferSimplifiedComparison);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(PreferSimplifiedComparison);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32, y: int32): bool {
    return x >= y
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-simplified-comparison");
    }
}
