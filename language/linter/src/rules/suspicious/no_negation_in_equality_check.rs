use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow negation in equality checks.
    ///
    /// Expressions like `!a == b` are confusing because the precedence makes
    /// it `(!a) == b` rather than `!(a == b)`. Use `a != b` or `!(a == b)` instead.
    #[lint(
        id = "no-negation-in-equality-check",
        code = "LU014",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoNegationInEqualityCheck,
    "Disallow negation in equality checks"
}

impl LintRule for NoNegationInEqualityCheck {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNegationInEqualityCheck::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // check for equality/inequality operators
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = expr
            else {
                continue;
            };

            // only check equality operators
            if !matches!(
                operator,
                ast::BinaryOperator::Equal
                    | ast::BinaryOperator::NotEqual
                    | ast::BinaryOperator::EqualStrict
                    | ast::BinaryOperator::NotEqualStrict
            ) {
                continue;
            }

            // check if left side is a negation
            let left_expr = ctx.tree.get(*left);
            if matches!(
                left_expr,
                ast::Expression::Unary {
                    operator: ast::UnaryOperator::Not,
                    ..
                }
            ) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_NEGATION_IN_EQUALITY_CHECK.id,
                        NO_NEGATION_IN_EQUALITY_CHECK.code,
                        NO_NEGATION_IN_EQUALITY_CHECK.category,
                        severity,
                        "negation in equality check is confusing",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use `!=` or wrap in parentheses `!(a == b)`"),
                );
            }

            // also check right side for symmetry
            let right_expr = ctx.tree.get(*right);
            if matches!(
                right_expr,
                ast::Expression::Unary {
                    operator: ast::UnaryOperator::Not,
                    ..
                }
            ) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_NEGATION_IN_EQUALITY_CHECK.id,
                        NO_NEGATION_IN_EQUALITY_CHECK.code,
                        NO_NEGATION_IN_EQUALITY_CHECK.category,
                        severity,
                        "negation in equality check is confusing",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use `!=` or wrap in parentheses"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_negation_on_left() {
        let test = TestProgram::for_rule(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = !a == b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_detects_negation_on_right() {
        let test = TestProgram::for_rule(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = a == !b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_detects_with_strict_equality() {
        let test = TestProgram::for_rule(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = !a === b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_allows_not_equal() {
        let test = TestProgram::for_rule(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = a != b
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_allows_negation_of_whole_expression() {
        let test = TestProgram::for_rule(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = !(a == b)
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_allows_normal_equality() {
        let test = TestProgram::for_rule(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = a == b
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
    }
}
