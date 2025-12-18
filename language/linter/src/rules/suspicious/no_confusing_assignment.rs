use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on assignments that look like comparisons.
    ///
    /// An assignment in a context where a comparison is expected (like `if (x = 1)`)
    /// is often a mistake. Use `===` for comparison or wrap in extra parentheses
    /// if assignment is intentional.
    #[lint(
        id = "no-confusing-assignment",
        code = "LU017",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoConfusingAssignment,
    "Warn on assignments that look like comparisons"
}

impl LintRule for NoConfusingAssignment {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConfusingAssignment::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check conditions in if/while/for that contain assignments
            let condition_id = match expression {
                ast::Expression::If { condition, .. } => Some(*condition),
                ast::Expression::While { condition, .. } => Some(*condition),
                ast::Expression::For {
                    condition: Some(condition),
                    ..
                } => Some(*condition),
                _ => None,
            };
            let Some(condition_id) = condition_id else {
                continue;
            };

            // check if condition is an assignment (not wrapped in extra parens)
            if is_bare_assignment(ctx, condition_id) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_CONFUSING_ASSIGNMENT.id,
                        NO_CONFUSING_ASSIGNMENT.code,
                        NO_CONFUSING_ASSIGNMENT.category,
                        severity,
                        "assignment in condition",
                        ctx.module.file_id,
                        ctx.tree.get_span(condition_id),
                    )
                    .with_label("use `===` for comparison or wrap assignment in extra parentheses"),
                );
            }
        }
    }
}

/// Return whether expression is a bare assignment (not doubly parenthesized).
fn is_bare_assignment(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expr_id);
    match expression {
        ast::Expression::Assign { .. } => true,
        ast::Expression::Parenthesized { expression } => {
            // single paren is still confusing, double parens is intentional
            let inner = ctx.tree.get(*expression);
            matches!(inner, ast::Expression::Assign { .. })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_assignment_in_if() {
        let test = TestProgram::for_rule(NoConfusingAssignment);
        let result = test.lint_ast("test.ts", "if (x = 1) {}");
        test.result(result).assert_lint("no-confusing-assignment");
    }

    #[test]
    fn test_detects_assignment_in_while() {
        let test = TestProgram::for_rule(NoConfusingAssignment);
        let result = test.lint_ast("test.ts", "while (x = next()) {}");
        test.result(result).assert_lint("no-confusing-assignment");
    }

    #[test]
    fn test_detects_single_paren_assignment() {
        let test = TestProgram::for_rule(NoConfusingAssignment);
        let result = test.lint_ast("test.ts", "if ((x = 1)) {}");
        test.result(result).assert_lint("no-confusing-assignment");
    }

    #[test]
    fn test_allows_double_paren_assignment() {
        let test = TestProgram::for_rule(NoConfusingAssignment);
        let result = test.lint_ast("test.ts", "if (((x = 1))) {}");
        test.result(result)
            .assert_no_lint("no-confusing-assignment");
    }

    #[test]
    fn test_allows_comparison() {
        let test = TestProgram::for_rule(NoConfusingAssignment);
        let result = test.lint_ast("test.ts", "if (x === 1) {}");
        test.result(result)
            .assert_no_lint("no-confusing-assignment");
    }

    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule(NoConfusingAssignment);
        let result = test.lint_ast("test.ts", "if (x) {}");
        test.result(result)
            .assert_no_lint("no-confusing-assignment");
    }
}
