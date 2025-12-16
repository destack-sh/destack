use destack_ast::{Expression, IfKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow nested ternary expressions.
    ///
    /// Nested ternary expressions like `a ? b ? c : d : e` are hard to read
    /// and understand. Consider using if-else statements or extracting logic
    /// into separate variables or functions.
    #[lint(
        id = "no-nested-ternary",
        code = "LY001",
        category = Style,
        level = Ast
    )]
    pub NoNestedTernary,
    "Disallow nested ternary expressions"
}

impl LintRule for NoNestedTernary {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNestedTernary::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<Expression>() {
            // look for ternary expressions
            let Expression::If {
                kind: IfKind::Ternary,
                condition,
                then_expression,
                else_expression,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // check if any child is also a ternary
            let has_nested_ternary = is_ternary(ctx, *condition)
                || is_ternary(ctx, *then_expression)
                || else_expression.map(|e| is_ternary(ctx, e)).unwrap_or(false);

            if has_nested_ternary {
                ctx.report(
                    LintDiagnostic::new(
                        NO_NESTED_TERNARY.id,
                        NO_NESTED_TERNARY.code,
                        NO_NESTED_TERNARY.category,
                        severity,
                        "nested ternary expression",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider using if-else instead"),
                );
            }
        }
    }
}

/// Check if an expression is a ternary (possibly wrapped in parentheses).
fn is_ternary(ctx: &LintModuleAstContext<'_>, expr_id: destack_ast::LocalNodeId<Expression>) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => true,
        Expression::Parenthesized { expression } => is_ternary(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_ternary_in_then() {
        let test = TestProgram::for_rule(NoNestedTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = a ? b ? 1 : 2 : 3;
"#,
        );
        test.result(result).assert_lint("no-nested-ternary");
    }

    #[test]
    fn test_detects_nested_ternary_in_else() {
        let test = TestProgram::for_rule(NoNestedTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = a ? 1 : b ? 2 : 3;
"#,
        );
        test.result(result).assert_lint("no-nested-ternary");
    }

    #[test]
    fn test_detects_nested_ternary_in_condition() {
        let test = TestProgram::for_rule(NoNestedTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = (a ? true : false) ? 1 : 2;
"#,
        );
        test.result(result).assert_lint("no-nested-ternary");
    }

    #[test]
    fn test_allows_simple_ternary() {
        let test = TestProgram::for_rule(NoNestedTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = condition ? 1 : 2;
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }

    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule(NoNestedTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = if (a) {
    if (b) { 1 } else { 2 }
} else {
    3
};
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }

    #[test]
    fn test_allows_separate_ternaries() {
        let test = TestProgram::for_rule(NoNestedTernary);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = a ? 1 : 2;
const y = b ? 3 : 4;
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }
}
