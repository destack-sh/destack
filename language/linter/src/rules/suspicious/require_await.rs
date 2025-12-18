use destack_ast::{self as ast, Asynchrony, Declaration, FunctionKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow async functions with no await expression.
    ///
    /// An async function without await is likely a mistake. The function
    /// will still return a Promise, but won't actually do async work.
    #[lint(
        id = "require-await",
        code = "LU024",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireAwait,
    "Require await in async functions"
}

impl LintRule for RequireAwait {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireAwait::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);

            let Declaration::Function {
                signature, body, ..
            } = declaration
            else {
                continue;
            };

            // only check async functions
            if signature.asynchrony != Asynchrony::Async {
                continue;
            }

            // skip lambda functions (arrow functions)
            if signature.kind == FunctionKind::Lambda {
                continue;
            }

            // skip functions without a body
            let Some(body_id) = body else {
                continue;
            };

            // check if body contains any await expression
            let has_await = contains_await(ctx, *body_id);

            if !has_await {
                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_AWAIT.id,
                        REQUIRE_AWAIT.code,
                        REQUIRE_AWAIT.category,
                        severity,
                        "async function has no await expression",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add await or remove async keyword"),
                );
            }
        }
    }
}

/// Check if an expression contains an await (recursively, but stop at function boundaries).
fn contains_await(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);

    match expr {
        ast::Expression::Await { .. } => true,
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            block.expressions.iter().any(|e| contains_await(ctx, *e))
        }
        ast::Expression::If {
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            contains_await(ctx, *condition)
                || contains_await(ctx, *then_expression)
                || else_expression.is_some_and(|e| contains_await(ctx, e))
        }
        ast::Expression::Binary { left, right, .. } => {
            contains_await(ctx, *left) || contains_await(ctx, *right)
        }
        ast::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } => {
            contains_await(ctx, *left)
                || dynamic_arguments.iter().any(|arg_id| {
                    let arg = ctx.tree.get(*arg_id);
                    match arg {
                        ast::Argument::Positional { value }
                        | ast::Argument::Spread { value }
                        | ast::Argument::Named { value, .. }
                        | ast::Argument::Labeled { value, .. } => contains_await(ctx, *value),
                    }
                })
        }
        ast::Expression::Statement(inner) => contains_await(ctx, *inner),
        ast::Expression::Parenthesized { expression } => contains_await(ctx, *expression),
        ast::Expression::Let { declarators, .. } => declarators.iter().any(|d_id| {
            let d = ctx.tree.get(*d_id);
            d.value.is_some_and(|v| contains_await(ctx, v))
        }),
        // stop at nested function declarations (they have their own async scope)
        ast::Expression::Declaration(_) => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_async_without_await_detected() {
        let test = TestProgram::for_rule(RequireAwait);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_lint("require-await");
    }

    #[test]
    fn test_async_with_await_allowed() {
        let test = TestProgram::for_rule(RequireAwait);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function foo() {
    let result = await fetch()
    return result
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_non_async_function_allowed() {
        let test = TestProgram::for_rule(RequireAwait);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }
}
