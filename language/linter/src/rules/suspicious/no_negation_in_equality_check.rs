use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow negation in equality checks.
    ///
    /// Expressions like `!a == b` are confusing because the precedence makes
    /// it `(!a) == b` rather than `!(a == b)`. Use `a != b` or `!(a == b)` instead.
    #[lint(
        id = "no-negation-in-equality-check",
        code = "LU024",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

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

            // determine which sides need explicit grouping
            let left_needs_grouping = needs_unary_not_grouping(ctx.tree.get(*left));
            let right_needs_grouping = needs_unary_not_grouping(ctx.tree.get(*right));
            if !left_needs_grouping && !right_needs_grouping {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build grouped expression replacement
            let left_span = ctx.tree.get_span(*left);
            let right_span = ctx.tree.get_span(*right);
            let mut left_text = ctx.get_span_text(left_span).to_string();
            let mut right_text = ctx.get_span_text(right_span).to_string();
            if left_needs_grouping {
                left_text = format!("({left_text})");
            }
            if right_needs_grouping {
                right_text = format!("({right_text})");
            }
            let replacement = format!(
                "{left_text} {} {right_text}",
                binary_operator_text(*operator)
            );
            let expression_span = ctx.tree.get_span(node_id);
            let edits = ctx
                .edit_builder()
                .replace(expression_span, replacement)
                .into_edits();
            let fix =
                LintFix::safe("Add explicit grouping around negated operand").with_edits(edits);

            ctx.report(
                LintDiagnostic::new(
                    NO_NEGATION_IN_EQUALITY_CHECK.id,
                    NO_NEGATION_IN_EQUALITY_CHECK.code,
                    NO_NEGATION_IN_EQUALITY_CHECK.category,
                    severity,
                    "negation in equality check is confusing",
                    ctx.module.file_id,
                    expression_span,
                )
                .with_label("add explicit grouping around the negated side")
                .with_fix(fix),
            );
        }
    }
}

/// Return true when one expression is an unparenthesized unary not.
fn needs_unary_not_grouping(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::Unary {
            operator: ast::UnaryOperator::Not,
            ..
        }
    )
}

/// Return source text for equality operators handled by this lint.
fn binary_operator_text(operator: ast::BinaryOperator) -> &'static str {
    match operator {
        ast::BinaryOperator::Equal => "==",
        ast::BinaryOperator::NotEqual => "!=",
        ast::BinaryOperator::EqualStrict => "===",
        ast::BinaryOperator::NotEqualStrict => "!==",
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_negation_on_left() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_detects_negation_on_left.ds",
            r#"
const x = !a == b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_has_fix("no-negation-in-equality-check");
    }

    #[test]
    fn test_detects_negation_on_right() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_detects_negation_on_right.ds",
            r#"
const x = a == !b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_detects_with_strict_equality() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_detects_with_strict_equality.ds",
            r#"
const x = !a === b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_allows_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_allows_not_equal.ds",
            r#"
const x = a != b
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_allows_negation_of_whole_expression() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_allows_negation_of_whole_expression.ds",
            r#"
const x = !(a == b)
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_allows_normal_equality() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_allows_normal_equality.ds",
            r#"
const x = a == b
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_fix_negation_on_left() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_fix_negation_on_left.ds",
            r#"
const x = !a == b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_safe_fixed(
                r#"
const x = (!a) == b;
"#,
            );
    }

    #[test]
    fn test_fix_negation_on_right() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_fix_negation_on_right.ds",
            r#"
const x = a == !b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_safe_fixed(
                r#"
const x = a == (!b);
"#,
            );
    }

    #[test]
    fn test_fix_negation_on_both_sides() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_fix_negation_on_both_sides.ds",
            r#"
const x = !a === !b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_lint_count("no-negation-in-equality-check", 1)
            .assert_safe_fixed(
                r#"
const x = (!a) === (!b);
"#,
            );
    }

    #[test]
    fn test_mutation_detects_not_equal_form() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_mutation_detects_not_equal_form.ds",
            r#"
const x = !a != b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }
}
