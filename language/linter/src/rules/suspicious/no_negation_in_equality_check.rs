use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{
    expression_can_start_expression_statement, expression_is_direct_statement,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow negation in equality checks.
    ///
    /// Expressions like `!a == b` are confusing because the precedence makes
    /// it `(!a) == b` rather than `!(a == b)`. Use `a != b` or `!(a == b)` instead.
    #[lint(
        id = "no-negation-in-equality-check",
        code = "LU024",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect binary expressions for confusing negated equality
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            // require a binary expression
            let expr = ctx.dir.get(node_id);
            let dir::Expression::Binary {
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
            let left_expression = ctx.dir.get(*left);
            let dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right: unary_argument_id,
            } = left_expression
            else {
                continue;
            };

            // skip double negation because intent is less clear
            let unary_argument = ctx.dir.get(*unary_argument_id);
            if matches!(
                unary_argument,
                dir::Expression::Unary {
                    operator: dir::UnaryOperator::Not,
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
            let right_span = ctx.dir.get_span(*right);
            let mut right_text = ctx.get_span_text(right_span).to_string();
            let unary_argument_span = ctx.dir.get_span(*unary_argument_id);
            let unary_argument_text = ctx.get_span_text(unary_argument_span).to_string();
            let expression_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_NEGATION_IN_EQUALITY_CHECK.id,
                NO_NEGATION_IN_EQUALITY_CHECK.code,
                NO_NEGATION_IN_EQUALITY_CHECK.category,
                severity,
                "negated expression in equality check is confusing",
                expression_span,
            )
            .label("remove `!` and invert the equality operator");

            // attach a fix when text extraction is stable
            if ctx.compute_fixes && !unary_argument_text.trim().is_empty() {
                // rewrite as direct comparison with the inverted operator
                let unary_argument_text = unary_argument_text.trim_start();
                right_text = right_text.trim_start().to_string();
                let unary_argument = ctx.dir.get(*unary_argument_id);
                if !expression_starts_unsafe_statement(ctx, node_id, unary_argument) {
                    let replacement = format!(
                        "{unary_argument_text} {} {right_text}",
                        equality_operator_text(fixed_operator)
                    );
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, replacement)
                        .into_edits();
                    let fix = LintFix::suggestion("Invert equality and remove leading negation")
                        .with_edits(edits);
                    diagnostic = diagnostic.fix(fix);
                }
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return one inverted operator for equality checks.
fn inverted_equality_operator(operator: dir::BinaryOperator) -> Option<dir::BinaryOperator> {
    match operator {
        dir::BinaryOperator::Equal => Some(dir::BinaryOperator::NotEqual),
        dir::BinaryOperator::NotEqual => Some(dir::BinaryOperator::Equal),
        dir::BinaryOperator::EqualStrict => Some(dir::BinaryOperator::NotEqualStrict),
        dir::BinaryOperator::NotEqualStrict => Some(dir::BinaryOperator::EqualStrict),
        _ => None,
    }
}

/// Return true when one fixed replacement would start an unsafe expression statement.
fn expression_starts_unsafe_statement(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    unary_argument: &dir::Expression,
) -> bool {
    if !expression_is_direct_statement(ctx.dir.tree(), expression_id) {
        return false;
    }

    !expression_can_start_expression_statement(unary_argument)
}

/// Return source text for equality operators handled by this lint.
fn equality_operator_text(operator: dir::BinaryOperator) -> &'static str {
    match operator {
        dir::BinaryOperator::Equal => "==",
        dir::BinaryOperator::NotEqual => "!=",
        dir::BinaryOperator::EqualStrict => "===",
        dir::BinaryOperator::NotEqualStrict => "!==",
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_negation_in_equality_check/test_fix_negation_on_left.ds",
            r#"
const x = !a == b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_suggested_fixed(
                r#"
const x = a != b;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_negation_on_right() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint(
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
        let result = test.lint(
            "no_negation_in_equality_check/test_fix_negation_on_both_sides.ds",
            r#"
const x = !a === !b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_lint_count("no-negation-in-equality-check", 1)
            .assert_suggested_fixed(
                r#"
const x = a !== !b;
"#,
            );
    }

    #[test]
    fn test_mutation_detects_not_equal_form() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint(
            "no_negation_in_equality_check/test_mutation_detects_not_equal_form.ds",
            r#"
const x = !a != b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check");
    }

    #[test]
    fn test_no_fix_for_asi_hazardous_statement_start() {
        let test = TestProgram::for_rule_without_prelude(NoNegationInEqualityCheck);
        let result = test.lint(
            "no_negation_in_equality_check/test_no_fix_for_asi_hazardous_statement_start.ds",
            r#"
foo
!(a) === b
"#,
        );
        test.result(result)
            .assert_lint("no-negation-in-equality-check")
            .assert_has_no_fix("no-negation-in-equality-check");
    }
}
