use crate::LintMeta;
use destack_ast::{self as ast, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_equal, expression_negated_source_text, source_text_contains_comment_token,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Disallow ternary operators that can be simplified.
    ///
    /// Patterns like `x ? true : false` or `x ? false : true` can be
    /// simplified to `x` or `!x` respectively.
    #[lint(
        id = "no-unneeded-ternary",
        code = "LY025",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnneededTernary,
    "Disallow unneeded ternary expressions"
}

impl LintRule for NoUnneededTernary {
    fn meta(&self) -> &'static LintMeta {
        NoUnneededTernary::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::If {
                condition,
                then_expression,
                else_expression: Some(else_expression),
                ..
            } = expr
            else {
                continue;
            };

            let condition_id = match condition {
                ast::IfCondition::Expression { condition } => *condition,
                ast::IfCondition::Let { .. } => continue,
            };
            let then_expr = ctx.tree.get(*then_expression);
            let else_expr = ctx.tree.get(*else_expression);
            let expression_span = ctx.tree.get_span(node_id);
            let condition_span = ctx.tree.get_span(condition_id);
            let condition_text = ctx.get_span_text(condition_span);
            let can_fix = !source_text_contains_comment_token(ctx.get_span_text(expression_span));

            // check for x ? true : false -> x
            if is_boolean_literal(then_expr, true) && is_boolean_literal(else_expr, false) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // make fix: replace `x ? true : false` with `x`
                let mut diagnostic = LintDiagnostic::new(
                    NO_UNNEEDED_TERNARY.id,
                    NO_UNNEEDED_TERNARY.code,
                    NO_UNNEEDED_TERNARY.category,
                    severity,
                    "unnecessary ternary `x ? true : false`",
                    ctx.module.file_id,
                    expression_span,
                )
                .with_label("use the condition directly");

                if can_fix {
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, condition_text)
                        .into_edits();
                    let fix = LintFix::safe("Simplify to condition").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
            // check for x ? false : true -> !x
            else if is_boolean_literal(then_expr, false) && is_boolean_literal(else_expr, true) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // make fix: replace `x ? false : true` with `!x`
                let mut diagnostic = LintDiagnostic::new(
                    NO_UNNEEDED_TERNARY.id,
                    NO_UNNEEDED_TERNARY.code,
                    NO_UNNEEDED_TERNARY.category,
                    severity,
                    "unnecessary ternary `x ? false : true`",
                    ctx.module.file_id,
                    expression_span,
                )
                .with_label("use `!x` instead");

                if can_fix {
                    let replacement = expression_negated_source_text(ctx, condition_id);
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, replacement)
                        .into_edits();
                    let fix = LintFix::safe("Simplify to negated condition").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
            // check for x ? x : y -> x || y when default-assignment simplification is enabled
            else if !ctx.options.style.no_unneeded_ternary_default_assignment
                && expression_is_equal(ctx, condition_id, *then_expression)
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let alternate_span = ctx.tree.get_span(*else_expression);
                let alternate_text = ctx.get_span_text(alternate_span);
                let mut diagnostic = LintDiagnostic::new(
                    NO_UNNEEDED_TERNARY.id,
                    NO_UNNEEDED_TERNARY.code,
                    NO_UNNEEDED_TERNARY.category,
                    severity,
                    "unnecessary ternary default assignment",
                    ctx.module.file_id,
                    expression_span,
                )
                .with_label("use `||` default assignment instead");

                if can_fix {
                    let replacement = format!("{condition_text} || {alternate_text}");
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, replacement)
                        .into_edits();
                    let fix = LintFix::safe("Simplify to default assignment").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

fn is_boolean_literal(expr: &ast::Expression, value: bool) -> bool {
    matches!(
        expr,
        ast::Expression::ScalarLiteral(ScalarLiteral::Boolean(v)) if *v == value
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_true_false() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_detects_true_false.ds",
            r#"
const result = x ? true : false
"#,
        );
        test.result(result).assert_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_detects_false_true() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_detects_false_true.ds",
            r#"
const result = x ? false : true
"#,
        );
        test.result(result).assert_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_allows_useful_ternary() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_allows_useful_ternary.ds",
            r#"
const result = x ? "yes" : "no"
"#,
        );
        test.result(result).assert_no_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_allows_if_else.ds",
            r#"
const result = if (x) { true } else { false }
"#,
        );
        // if-else block, not ternary
        test.result(result).assert_no_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_fix_true_false() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_fix_true_false.ds",
            r#"
const result = x ? true : false
"#,
        );
        test.result(result)
            .assert_lint("no-unneeded-ternary")
            .assert_safe_fixed(
                r#"
const result = x;
"#,
            );
    }

    #[test]
    fn test_has_no_fix_when_expression_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_has_no_fix_when_expression_contains_comment.ds",
            r#"
const result = x ? /* keep */ true : false
"#,
        );
        test.result(result)
            .assert_lint("no-unneeded-ternary")
            .assert_has_no_fix("no-unneeded-ternary");
    }

    #[test]
    fn test_fix_false_true() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_fix_false_true.ds",
            r#"
const result = x ? false : true
"#,
        );
        test.result(result)
            .assert_lint("no-unneeded-ternary")
            .assert_safe_fixed(
                r#"
const result = !x;
"#,
            );
    }

    #[test]
    fn test_fix_false_true_with_compound_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_fix_false_true_with_compound_condition.ds",
            r#"
const result = a && b ? false : true
"#,
        );
        test.result(result)
            .assert_lint("no-unneeded-ternary")
            .assert_safe_fixed(
                r#"
const result = !(a && b);
"#,
            );
    }

    #[test]
    fn test_allows_default_assignment_ternary_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_allows_default_assignment_ternary_by_default.ds",
            r#"
const result = value ? value : fallback
"#,
        );
        test.result(result).assert_no_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_flags_default_assignment_ternary_when_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoUnneededTernary)
            .with_options(|options| options.style.no_unneeded_ternary_default_assignment = false);
        let result = test.lint_ast(
            "no_unneeded_ternary/test_flags_default_assignment_ternary_when_disabled.ds",
            r#"
const result = value ? value : fallback
"#,
        );
        test.result(result)
            .assert_lint("no-unneeded-ternary")
            .assert_safe_fixed(
                r#"
const result = value || fallback;
"#,
            );
    }
}
