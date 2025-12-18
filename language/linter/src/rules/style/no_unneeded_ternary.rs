use destack_ast::{self as ast, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = No,
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
                kind: ast::IfKind::Ternary,
                then_expression,
                else_expression: Some(else_expression),
                ..
            } = expr
            else {
                continue;
            };

            let then_expr = ctx.tree.get(*then_expression);
            let else_expr = ctx.tree.get(*else_expression);

            // check for x ? true : false
            if is_boolean_literal(then_expr, true) && is_boolean_literal(else_expr, false) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_UNNEEDED_TERNARY.id,
                        NO_UNNEEDED_TERNARY.code,
                        NO_UNNEEDED_TERNARY.category,
                        severity,
                        "unnecessary ternary `x ? true : false`",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use the condition directly"),
                );
            }
            // check for x ? false : true
            else if is_boolean_literal(then_expr, false) && is_boolean_literal(else_expr, true) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_UNNEEDED_TERNARY.id,
                        NO_UNNEEDED_TERNARY.code,
                        NO_UNNEEDED_TERNARY.category,
                        severity,
                        "unnecessary ternary `x ? false : true`",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use `!x` instead"),
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
}
