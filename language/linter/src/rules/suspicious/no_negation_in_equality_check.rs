use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

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
    fn meta(&self) -> &'static LintMeta {
        NoNegationInEqualityCheck::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect binary expressions for confusing negated equality
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // require a binary expression
            let expr = ctx.tree.get(node_id);
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = expr
            else {
                continue;
            };

            // keep only equality operators that can be inverted
            let Some(fixed_operator) = inverted_equality_operator(*operator) else {
                continue;
            };

            // require a leading unary not on the left side
            let left_expression = ctx.tree.get(*left);
            let ast::Expression::Unary {
                operator: ast::UnaryOperator::Not,
                right: unary_argument_id,
            } = left_expression
            else {
                continue;
            };

            // skip double negation because intent is less clear
            let unary_argument = ctx.tree.get(*unary_argument_id);
            if matches!(
                unary_argument,
                ast::Expression::Unary {
                    operator: ast::UnaryOperator::Not,
                    ..
                }
            ) {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the replacement inputs and diagnostic payload
            let right_span = ctx.tree.get_span(*right);
            let mut right_text = ctx.get_span_text(right_span).to_string();
            let unary_argument_span = ctx.tree.get_span(*unary_argument_id);
            let mut unary_argument_text = ctx.get_span_text(unary_argument_span).to_string();
            let expression_span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_NEGATION_IN_EQUALITY_CHECK.id,
                NO_NEGATION_IN_EQUALITY_CHECK.code,
                NO_NEGATION_IN_EQUALITY_CHECK.category,
                severity,
                "negated expression in equality check is confusing",
                ctx.module.file_id,
                expression_span,
            )
            .with_label("remove `!` and invert the equality operator");

            // attach a fix when text extraction is stable
            if ctx.compute_fixes && !unary_argument_text.trim().is_empty() {
                // guard token boundaries when removing the leading unary not
                if needs_leading_fix_space(ctx, expression_span.start) {
                    unary_argument_text = format!(" {unary_argument_text}");
                }

                // rewrite as direct comparison with the inverted operator
                right_text = right_text.trim_start().to_string();
                let replacement = format!(
                    "{unary_argument_text} {} {right_text}",
                    equality_operator_text(fixed_operator)
                );
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix =
                    LintFix::safe("Invert equality and remove leading negation").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return one inverted operator for equality checks.
fn inverted_equality_operator(operator: ast::BinaryOperator) -> Option<ast::BinaryOperator> {
    match operator {
        ast::BinaryOperator::Equal => Some(ast::BinaryOperator::NotEqual),
        ast::BinaryOperator::NotEqual => Some(ast::BinaryOperator::Equal),
        ast::BinaryOperator::EqualStrict => Some(ast::BinaryOperator::NotEqualStrict),
        ast::BinaryOperator::NotEqualStrict => Some(ast::BinaryOperator::EqualStrict),
        _ => None,
    }
}

/// Return true when one replacement needs a leading space for lexical safety.
fn needs_leading_fix_space(ctx: &LintAstContext<'_>, start: u32) -> bool {
    if start == 0 {
        return false;
    }

    // preserve token separation when the previous character is identifier like
    let source = ctx.source_text().as_bytes();
    let previous_byte = source[(start - 1) as usize];
    previous_byte.is_ascii_alphanumeric() || previous_byte == b'_' || previous_byte == b'$'
}

/// Return source text for equality operators handled by this lint.
fn equality_operator_text(operator: ast::BinaryOperator) -> &'static str {
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
            .assert_no_lint("no-negation-in-equality-check");
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
const x = a != b;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_negation_on_right() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint_ast(
            "no_negation_in_equality_check/test_no_fix_for_negation_on_right.ds",
            r#"
const x = a == !b
"#,
        );
        test.result(result)
            .assert_no_lint("no-negation-in-equality-check");
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
const x = a !== !b;
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
