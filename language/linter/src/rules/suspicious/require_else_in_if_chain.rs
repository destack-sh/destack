use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require final else in if-else-if chains.
    ///
    /// An if-else-if chain without a final else clause may indicate
    /// missing logic for unhandled cases.
    #[lint(
        id = "require-else-in-if-chain",
        code = "LU025",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireElseInIfChain,
    "Require else in if chains"
}

impl LintRule for RequireElseInIfChain {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireElseInIfChain::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::If {
                else_expression: Some(else_expr),
                ..
            } = expression
            else {
                continue;
            };

            // check if the else branch is another if (making this an if-else-if chain)
            let else_branch = ctx.tree.get(*else_expr);

            // unwrap block if needed
            let else_branch = if let ast::Expression::Block(block_id) = else_branch {
                let block = ctx.tree.get(*block_id);
                if block.expressions.len() == 1 {
                    ctx.tree.get(block.expressions[0])
                } else {
                    continue;
                }
            } else {
                else_branch
            };

            // unwrap statement if needed
            let else_branch = if let ast::Expression::Statement(inner) = else_branch {
                ctx.tree.get(*inner)
            } else {
                else_branch
            };

            // check if else branch is an if without else (end of chain without final else)
            if let ast::Expression::If {
                else_expression: None,
                ..
            } = else_branch
            {
                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_ELSE_IN_IF_CHAIN.id,
                        REQUIRE_ELSE_IN_IF_CHAIN.code,
                        REQUIRE_ELSE_IN_IF_CHAIN.category,
                        severity,
                        "if-else-if chain lacks final else clause",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add final else clause"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_if_else_if_without_else_detected() {
        let test = TestProgram::for_rule(RequireElseInIfChain);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else if (x < 0) {
        console.log("negative")
    }
}
"#,
        );
        test.result(result).assert_lint("require-else-in-if-chain");
    }

    #[test]
    fn test_if_else_if_else_allowed() {
        let test = TestProgram::for_rule(RequireElseInIfChain);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else if (x < 0) {
        console.log("negative")
    } else {
        console.log("zero")
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }

    #[test]
    fn test_simple_if_allowed() {
        let test = TestProgram::for_rule(RequireElseInIfChain);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }

    #[test]
    fn test_simple_if_else_allowed() {
        let test = TestProgram::for_rule(RequireElseInIfChain);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else {
        console.log("not positive")
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }
}
