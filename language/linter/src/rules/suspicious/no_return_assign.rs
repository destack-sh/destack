use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignment operators in return statements.
    ///
    /// Assignments in return statements are often mistakes where `=` was typed
    /// instead of `==`. If intentional, separate the assignment from the return.
    #[lint(
        id = "no-return-assign",
        code = "LU015",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoReturnAssign,
    "Disallow assignment in return statements"
}

impl LintRule for NoReturnAssign {
    fn meta(&self) -> &'static crate::LintMeta {
        NoReturnAssign::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Return {
                value: Some(value_id),
            } = expression
            else {
                continue;
            };

            // check if the return value is an assignment
            if contains_assignment(ctx, *value_id) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_RETURN_ASSIGN.id,
                        NO_RETURN_ASSIGN.code,
                        NO_RETURN_ASSIGN.category,
                        severity,
                        "assignment in return statement",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("separate assignment from return"),
                );
            }
        }
    }
}

/// Return whether the expression contains an assignment.
fn contains_assignment(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expr_id);
    match expression {
        ast::Expression::Assign { .. } => true,
        ast::Expression::Parenthesized { expression } => contains_assignment(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_return_assignment() {
        let test = TestProgram::for_rule(NoReturnAssign);
        let result = test.lint_ast("test.ts", "function foo() { return x = 1; }");
        test.result(result).assert_lint("no-return-assign");
    }

    #[test]
    fn test_detects_parenthesized_assignment() {
        let test = TestProgram::for_rule(NoReturnAssign);
        let result = test.lint_ast("test.ts", "function foo() { return (x = 1); }");
        test.result(result).assert_lint("no-return-assign");
    }

    #[test]
    fn test_allows_normal_return() {
        let test = TestProgram::for_rule(NoReturnAssign);
        let result = test.lint_ast("test.ts", "function foo() { return x; }");
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_allows_comparison_in_return() {
        let test = TestProgram::for_rule(NoReturnAssign);
        let result = test.lint_ast("test.ts", "function foo() { return x == 1; }");
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_allows_empty_return() {
        let test = TestProgram::for_rule(NoReturnAssign);
        let result = test.lint_ast("test.ts", "function foo() { return; }");
        test.result(result).assert_no_lint("no-return-assign");
    }
}
