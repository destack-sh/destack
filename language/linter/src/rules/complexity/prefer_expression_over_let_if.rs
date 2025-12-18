use destack_ast::{self as ast, Pattern};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest expression syntax over let-if sequences.
    ///
    /// Instead of declaring a variable and then assigning in branches,
    /// use an if expression or ternary to initialize directly.
    #[lint(
        id = "prefer-expression-over-let-if",
        code = "LX013",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferExpressionOverLetIf,
    "Prefer expression over let-if"
}

/// Get the binding name from a pattern if it's a simple identifier.
fn get_binding_name(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<Pattern>,
) -> Option<destack_base::StringId> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some(*name),
        _ => None,
    }
}

/// Check if an expression is an assignment to a specific variable.
fn is_assignment_to(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    target_name: destack_base::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);

    // unwrap Statement wrapper
    let expr = match expr {
        ast::Expression::Statement(inner) => ctx.tree.get(*inner),
        other => other,
    };

    match expr {
        ast::Expression::Assign { left, .. } => {
            // check if left is the same variable
            let left_expr = ctx.tree.get(*left);
            if let ast::Expression::Path { path, .. } = left_expr
                && path.segments.len() == 1
            {
                return path.segments[0] == target_name;
            }
            false
        }
        _ => false,
    }
}

/// Check if an expression (which should be a block) contains only an assignment to the target.
fn expr_is_simple_assignment(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    target_name: destack_base::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);

    // if it's a block expression, check its contents
    if let ast::Expression::Block(block_id) = expr {
        let block = ctx.tree.get(*block_id);
        if block.expressions.len() == 1 {
            return is_assignment_to(ctx, block.expressions[0], target_name);
        }
    }

    false
}

impl LintRule for PreferExpressionOverLetIf {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferExpressionOverLetIf::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // look for blocks with potential let-if patterns
        for block_id in ctx.tree.iter_nodes::<ast::Block>() {
            let block = ctx.tree.get(block_id);

            // need at least 2 expressions
            if block.expressions.len() < 2 {
                continue;
            }

            // check consecutive pairs
            for i in 0..block.expressions.len() - 1 {
                let first_id = block.expressions[i];
                let second_id = block.expressions[i + 1];

                // unwrap statement wrappers
                let first_expr = unwrap_statement(ctx, first_id);
                let second_expr = unwrap_statement(ctx, second_id);

                // first must be a let without initializer
                let ast::Expression::Let { declarators, .. } = first_expr else {
                    continue;
                };

                // must be a single declarator without value
                if declarators.len() != 1 {
                    continue;
                }
                let decl = ctx.tree.get(declarators[0]);
                if decl.value.is_some() {
                    continue;
                }

                // get the binding name
                let Some(var_name) = get_binding_name(ctx, decl.pattern) else {
                    continue;
                };

                // second must be an if-else that assigns to the variable in both branches
                let ast::Expression::If {
                    then_expression,
                    else_expression: Some(else_expr),
                    ..
                } = second_expr
                else {
                    continue;
                };

                // both branches must be simple assignments to the variable
                if !expr_is_simple_assignment(ctx, *then_expression, var_name) {
                    continue;
                }
                if !expr_is_simple_assignment(ctx, *else_expr, var_name) {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_EXPRESSION_OVER_LET_IF.id,
                        PREFER_EXPRESSION_OVER_LET_IF.code,
                        PREFER_EXPRESSION_OVER_LET_IF.category,
                        severity,
                        "prefer expression syntax over let-if sequence",
                        ctx.module.file_id,
                        ctx.tree.get_span(first_id),
                    )
                    .with_label("use if expression to initialize directly"),
                );
            }
        }
    }
}

fn unwrap_statement<'a>(
    ctx: &'a LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> &'a ast::Expression {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Statement(inner) => ctx.tree.get(*inner),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_let_if_sequence_detected() {
        let test = TestProgram::for_rule(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(cond: bool) {
    let result: int32
    if (cond) {
        result = 1
    } else {
        result = 2
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-expression-over-let-if");
    }

    #[test]
    fn test_direct_expression_allowed() {
        let test = TestProgram::for_rule(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(cond: bool) {
    let result = cond ? 1 : 2
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-expression-over-let-if");
    }

    #[test]
    fn test_let_with_initializer_allowed() {
        let test = TestProgram::for_rule(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(cond: bool) {
    let result = 0
    if (cond) {
        result = 1
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-expression-over-let-if");
    }

    #[test]
    fn test_if_without_else_allowed() {
        let test = TestProgram::for_rule(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(cond: bool) {
    let result: int32
    if (cond) {
        result = 1
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-expression-over-let-if");
    }
}
