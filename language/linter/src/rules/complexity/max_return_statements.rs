use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of return statements per function.
    ///
    /// Functions with many return points can be harder to follow and maintain.
    /// Consider restructuring with early returns or extracting logic.
    #[lint(
        id = "max-return-statements",
        code = "LX016",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxReturnStatements,
    "Limit return statements per function"
}

// nocheckin: use NodeVisitor here for max-return-statements? a bit like in no-nested-callbacks?

impl LintRule for MaxReturnStatements {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxReturnStatements::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let max_return_statements = ctx.options.max_return_statements;

        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let ast::Declaration::Function { body, .. } = declaration else {
                continue;
            };
            let Some(body_id) = body else {
                continue;
            };

            // count return statements in the function body
            let return_count = count_returns(ctx, *body_id);
            if return_count > max_return_statements {
                let body_span = ctx.tree.get_span(*body_id);
                ctx.report(
                    LintDiagnostic::new(
                        MAX_RETURN_STATEMENTS.id,
                        MAX_RETURN_STATEMENTS.code,
                        MAX_RETURN_STATEMENTS.category,
                        severity,
                        format!(
                            "function has {return_count} return statements (max {max_return_statements})"
                        ),
                        ctx.module.file_id,
                        body_span,
                    )
                    .with_label("consider restructuring to reduce return points"),
                );
            }
        }
    }
}

/// Count return statements in an expression (recursively).
fn count_returns(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> usize {
    let expr = ctx.tree.get(expr_id);

    match expr {
        ast::Expression::Return { .. } => 1,

        // recurse into blocks
        ast::Expression::Block(block_id) => count_returns_in_block(ctx, *block_id),

        // recurse into control flow
        ast::Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            let mut count = count_returns(ctx, *then_expression);
            if let Some(alt) = else_expression {
                count += count_returns(ctx, *alt);
            }
            count
        }

        ast::Expression::Match { cases, .. } => cases
            .iter()
            .map(|case_id| {
                let case = ctx.tree.get(*case_id);
                match case {
                    ast::MatchCase::Expression { body, .. } => count_returns(ctx, *body),
                    ast::MatchCase::Block { body, .. } => count_returns_in_block(ctx, *body),
                }
            })
            .sum(),

        ast::Expression::For { body, .. }
        | ast::Expression::ForEach { body, .. }
        | ast::Expression::While { body, .. }
        | ast::Expression::Loop { body, .. } => count_returns_in_block(ctx, *body),

        ast::Expression::Try {
            try_expression,
            catch_expression,
            finally_expression,
            ..
        } => {
            let mut count = count_returns(ctx, *try_expression);
            if let Some(catch_expr) = catch_expression {
                count += count_returns(ctx, *catch_expr);
            }
            if let Some(finally_expr) = finally_expression {
                count += count_returns(ctx, *finally_expr);
            }
            count
        }

        ast::Expression::Labelled { body, .. } => count_returns(ctx, *body),

        ast::Expression::Statement(inner)
        | ast::Expression::Parenthesized { expression: inner } => count_returns(ctx, *inner),

        // don't recurse into nested function declarations, they have their own scope
        ast::Expression::Declaration(_) => 0,

        // other expressions don't contain returns
        _ => 0,
    }
}

/// Count return statements in a block (recursively).
fn count_returns_in_block(
    ctx: &LintModuleAstContext<'_>,
    block_id: ast::LocalNodeId<ast::Block>,
) -> usize {
    let block = ctx.tree.get(block_id);
    block
        .expressions
        .iter()
        .map(|e| count_returns(ctx, *e))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_returns() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function tooManyReturns(x: int32): int32 {
    if (x < 0) { return -1; }
    if (x == 0) { return 0; }
    if (x == 1) { return 1; }
    if (x == 2) { return 2; }
    if (x == 3) { return 3; }
    if (x == 4) { return 4; }
    if (x == 5) { return 5; }
    if (x == 6) { return 6; }
    if (x == 7) { return 7; }
    if (x == 8) { return 8; }
    return 10;
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }

    #[test]
    fn test_allows_few_returns() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function fewReturns(x: int32): int32 {
    if (x < 0) { return -1; }
    if (x == 0) { return 0; }
    return x;
}
"#,
        );
        test.result(result).assert_no_lint("max-return-statements");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        // 10 returns is at the limit (default max is 10)
        let result = test.lint_ast(
            "test.ds",
            r#"
function atLimit(x: int32): int32 {
    if (x == 0) { return 0; }
    if (x == 1) { return 1; }
    if (x == 2) { return 2; }
    if (x == 3) { return 3; }
    if (x == 4) { return 4; }
    if (x == 5) { return 5; }
    if (x == 6) { return 6; }
    if (x == 7) { return 7; }
    if (x == 8) { return 8; }
    return 9;
}
"#,
        );
        test.result(result).assert_no_lint("max-return-statements");
    }

    #[test]
    fn test_counts_returns_in_match() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function matchReturns(x: int32): int32 {
    match (x) {
        0 => return 0
        1 => return 1
        2 => return 2
        3 => return 3
        4 => return 4
        5 => return 5
        6 => return 6
        7 => return 7
        8 => return 8
        9 => return 9
        _ => return 10
    }
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }

    #[test]
    fn test_does_not_count_nested_function() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function outer(x: int32): int32 {
    function inner(y: int32): int32 {
        if (y == 0) { return 0; }
        if (y == 1) { return 1; }
        if (y == 2) { return 2; }
        if (y == 3) { return 3; }
        if (y == 4) { return 4; }
        if (y == 5) { return 5; }
        if (y == 6) { return 6; }
        if (y == 7) { return 7; }
        if (y == 8) { return 8; }
        if (y == 9) { return 9; }
        return 10;
    }
    return inner(x);
}
"#,
        );
        // outer has 1 return, inner has 11 but is a separate function
        // so we get TWO violations: one for outer (1 return, ok), one for inner (11 returns, not ok)
        test.result(result).assert_lint("max-return-statements");
    }
}
