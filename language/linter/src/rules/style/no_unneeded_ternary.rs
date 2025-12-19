use destack_ast::{self as ast, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow ternary operators that can be simplified.
    ///
    /// Patterns like `x ? true : false` or `x ? false : true` can be
    /// simplified to `x` or `!x` respectively.
    #[lint(
        id = "no-unneeded-ternary",
        code = "LY013",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnneededTernary,
    "Disallow unneeded ternary expressions"
}

impl LintRule for NoUnneededTernary {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnneededTernary::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
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

            let then_expr = ctx.tree.get(*then_expression);
            let else_expr = ctx.tree.get(*else_expression);
            let expression_span = ctx.tree.get_span(node_id);
            let condition_span = ctx.tree.get_span(*condition);
            let condition_text = ctx.get_span_text(condition_span);

            // check for x ? true : false -> x
            if is_boolean_literal(then_expr, true) && is_boolean_literal(else_expr, false) {
                // make fix: replace `x ? true : false` with `x`
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, condition_text)
                    .into_edits();
                let fix = LintFix::safe("Simplify to condition").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_UNNEEDED_TERNARY.id,
                        NO_UNNEEDED_TERNARY.code,
                        NO_UNNEEDED_TERNARY.category,
                        severity,
                        "unnecessary ternary `x ? true : false`",
                        ctx.module.file_id,
                        expression_span,
                    )
                    .with_label("use the condition directly")
                    .with_fix(fix),
                );
            }
            // check for x ? false : true -> !x
            else if is_boolean_literal(then_expr, false) && is_boolean_literal(else_expr, true) {
                // make fix: replace `x ? false : true` with `!x`
                let replacement = format!("!{condition_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Simplify to negated condition").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_UNNEEDED_TERNARY.id,
                        NO_UNNEEDED_TERNARY.code,
                        NO_UNNEEDED_TERNARY.category,
                        severity,
                        "unnecessary ternary `x ? false : true`",
                        ctx.module.file_id,
                        expression_span,
                    )
                    .with_label("use `!x` instead")
                    .with_fix(fix),
                );
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
        let test = TestProgram::for_rule(NoUnneededTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x ? true : false
"#,
        );
        test.result(result).assert_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_detects_false_true() {
        let test = TestProgram::for_rule(NoUnneededTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x ? false : true
"#,
        );
        test.result(result).assert_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_allows_useful_ternary() {
        let test = TestProgram::for_rule(NoUnneededTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x ? "yes" : "no"
"#,
        );
        test.result(result).assert_no_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule(NoUnneededTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = if (x) { true } else { false }
"#,
        );
        // if-else block, not ternary
        test.result(result).assert_no_lint("no-unneeded-ternary");
    }

    #[test]
    fn test_fix_true_false() {
        let test = TestProgram::for_rule(NoUnneededTernary);
        let result = test.lint_ast(
            "test.ds",
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
    fn test_fix_false_true() {
        let test = TestProgram::for_rule(NoUnneededTernary);
        let result = test.lint_ast(
            "test.ds",
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
}
