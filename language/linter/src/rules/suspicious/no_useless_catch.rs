use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow catch clauses that only rethrow the caught error.
    ///
    /// A catch clause that only rethrows the original error is useless.
    /// The same behavior can be achieved by removing the try-catch entirely.
    #[lint(
        id = "no-useless-catch",
        code = "LU002",
        category = Suspicious,
        level = Ast
    )]
    pub NoUselessCatch,
    "Disallow catch that just rethrows"
}

impl LintRule for NoUselessCatch {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessCatch::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Try {
                catch_pattern,
                catch_expression,
                finally_expression,
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // need both a catch pattern and expression
            let (Some(pattern_id), Some(catch_expr_id)) = (catch_pattern, catch_expression) else {
                continue;
            };

            // get the bound name from the catch pattern
            let Some(catch_name) = get_pattern_binding_name(ctx, *pattern_id) else {
                continue;
            };

            // check if the catch expression just throws the caught variable
            if is_throw_of_name(ctx, *catch_expr_id, catch_name) {
                // if there's a finally block, the catch might still be useful for cleanup ordering
                if finally_expression.is_some() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_CATCH.id,
                        NO_USELESS_CATCH.code,
                        NO_USELESS_CATCH.category,
                        severity,
                        "useless catch clause",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this catch just rethrows the error"),
                );
            }
        }
    }
}

/// Get the binding name from a simple catch pattern.
fn get_pattern_binding_name(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::StringId> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Binding { name, .. } => Some(*name),
        _ => None,
    }
}

/// Check if an expression is `throw <name>` where name matches the given string id.
fn is_throw_of_name(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    name: ast::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // direct throw
        ast::Expression::Throw { value } => is_path_to_name(ctx, *value, name),
        // block with single throw statement
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            if block.expressions.len() == 1 {
                is_throw_of_name(ctx, block.expressions[0], name)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if an expression is a simple path referencing the given name.
fn is_path_to_name(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    name: ast::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Path { path, .. } => {
            // simple identifier path (single segment, not absolute)
            path.segments.len() == 1 && path.segments[0] == name
        }
        ast::Expression::Parenthesized { expression } => is_path_to_name(ctx, *expression, name),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_useless_catch_throw() {
        let test = TestProgram::for_rule(NoUselessCatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    foo()
} catch e {
    throw e
}
"#,
        );
        test.result(result).assert_lint("no-useless-catch");
    }

    #[test]
    fn test_detects_useless_catch_block_throw() {
        let test = TestProgram::for_rule(NoUselessCatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    foo()
} catch err {
    throw err
}
"#,
        );
        test.result(result).assert_lint("no-useless-catch");
    }

    #[test]
    fn test_allows_catch_with_logging() {
        let test = TestProgram::for_rule(NoUselessCatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    console.log(e);
    throw e;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-catch");
    }

    #[test]
    fn test_allows_catch_with_different_throw() {
        let test = TestProgram::for_rule(NoUselessCatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    throw new Error("wrapped");
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-catch");
    }

    #[test]
    fn test_allows_catch_with_finally() {
        // catch + finally might be useful for cleanup ordering
        let test = TestProgram::for_rule(NoUselessCatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    throw e;
} finally {
    cleanup();
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-catch");
    }
}
