use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignment expressions in conditional statements.
    ///
    /// Using an assignment in a condition is often a mistake: `if (x = 1)` was
    /// probably meant to be `if (x == 1)`. If assignment is intentional, wrap
    /// it in parentheses: `if ((x = getValue()))`.
    #[lint(
        id = "no-cond-assign",
        code = "LU001",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoCondAssign,
    "Disallow assignment in conditions"
}

impl LintRule for NoCondAssign {
    fn meta(&self) -> &'static crate::LintMeta {
        NoCondAssign::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let condition_id = match ctx.tree.get(node_id) {
                ast::Expression::If { condition, .. } => *condition,
                ast::Expression::While { condition, .. } => *condition,
                // don't check for-loop conditions since `for (;x=y;)` is less common
                _ => continue,
            };

            // check if the condition is an assignment (possibly wrapped in parens)
            if is_assignment(ctx, condition_id) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let assign_span = ctx.tree.get_span(condition_id);

                ctx.report(
                    LintDiagnostic::new(
                        NO_COND_ASSIGN.id,
                        NO_COND_ASSIGN.code,
                        NO_COND_ASSIGN.category,
                        severity,
                        "assignment in condition",
                        ctx.module.file_id,
                        assign_span,
                    )
                    .with_label("did you mean `==`?"),
                );
            }
        }
    }
}

/// Check if an expression is an assignment (possibly wrapped in parentheses).
/// Let expressions are allowed (like Rust's `if let`).
fn is_assignment(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // let expressions are allowed (like `if const Some(x) = foo()`)
        ast::Expression::Let { .. } => false,
        ast::Expression::Assign { .. } => true,
        ast::Expression::Parenthesized { expression } => is_assignment(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_if_assignment() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x = 1) {
    console.log(x);
}
"#,
        );
        test.result(result).assert_lint("no-cond-assign");
    }

    #[test]
    fn test_detects_while_assignment() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (x = getValue()) {
    process(x);
}
"#,
        );
        test.result(result).assert_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_comparison() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x == 1) {
    console.log(x);
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_strict_comparison() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x === 1) {
    console.log(x);
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (isReady) {
    start();
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_function_call_condition() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (hasMore()) {
    processNext()
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_let_expression_in_condition() {
        let test = TestProgram::for_rule(NoCondAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (const x = getValue()) {
    process(x)
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }
}
