use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow loops that execute at most once.
    ///
    /// A loop that always exits on the first iteration is likely a bug.
    /// This happens when the loop body unconditionally contains a
    /// return, break, throw, or continue statement.
    ///
    /// ## Bad
    /// ```
    /// for (item in items) {
    ///     return item;  // always exits on first iteration
    /// }
    /// ```
    ///
    /// ## Good
    /// ```
    /// for (item in items) {
    ///     if (item.matches) {
    ///         return item;  // conditional exit
    ///     }
    /// }
    /// ```
    #[lint(
        id = "no-loop-single-iteration",
        code = "LC020",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoLoopSingleIteration,
    "Disallow loops that execute at most once"
}

impl LintRule for NoLoopSingleIteration {
    fn meta(&self) -> &'static crate::LintMeta {
        NoLoopSingleIteration::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check various loop types
            let body_id = match expression {
                ast::Expression::For { body, .. } => *body,
                ast::Expression::ForEach { body, .. } => *body,
                ast::Expression::While { body, .. } => *body,
                ast::Expression::Loop { body, .. } => *body,
                _ => continue,
            };

            // check if body unconditionally exits
            if body_unconditionally_exits(ctx, body_id) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let loop_type = match expression {
                    ast::Expression::For { .. } => "for",
                    ast::Expression::ForEach { .. } => "for-each",
                    ast::Expression::While { .. } => "while",
                    ast::Expression::Loop { .. } => "loop",
                    _ => "loop",
                };

                ctx.report(
                    LintDiagnostic::new(
                        NO_LOOP_SINGLE_ITERATION.id,
                        NO_LOOP_SINGLE_ITERATION.code,
                        NO_LOOP_SINGLE_ITERATION.category,
                        severity,
                        format!("{loop_type} loop executes at most once"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("body unconditionally exits on first iteration"),
                );
            }
        }
    }
}

/// Check if a block unconditionally exits (return, break, throw, continue).
fn body_unconditionally_exits(
    ctx: &LintModuleAstContext<'_>,
    body_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    let block = ctx.tree.get(body_id);

    // empty block doesn't exit
    if block.expressions.is_empty() {
        return false;
    }

    // check all expressions - if ANY expression unconditionally exits, the loop exits
    for expr_id in &block.expressions {
        if expression_unconditionally_exits(ctx, *expr_id) {
            return true;
        }
    }

    false
}

/// Check if an expression unconditionally exits.
fn expression_unconditionally_exits(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);

    match expr {
        // direct exit statements
        ast::Expression::Return { .. } => true,
        ast::Expression::Break { .. } => true,
        ast::Expression::Continue { .. } => true,
        ast::Expression::Throw { .. } => true,

        // unwrap statement wrapper
        ast::Expression::Statement(inner_id) => expression_unconditionally_exits(ctx, *inner_id),

        // block: check if it unconditionally exits
        ast::Expression::Block(block_id) => body_unconditionally_exits(ctx, *block_id),

        // if/else: only exits if BOTH branches exit (we can't determine this easily for all branches)
        // so we don't flag conditional exits
        ast::Expression::If { .. } => false,

        // match: would need to check all arms, skip for now
        ast::Expression::Match { .. } => false,

        // try: complex control flow, skip
        ast::Expression::Try { .. } => false,

        // other expressions don't exit
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_for_with_unconditional_return() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < 10; i++) {
    return i;
}
"#,
        );
        test.result(result).assert_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_detects_foreach_with_unconditional_return() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (item in items) {
    return item;
}
"#,
        );
        test.result(result).assert_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_detects_while_with_unconditional_break() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (true) {
    break;
}
"#,
        );
        test.result(result).assert_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_detects_loop_with_unconditional_throw() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
loop {
    throw Error("oops");
}
"#,
        );
        test.result(result).assert_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_conditional_return() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (item in items) {
    if (item.matches) {
        return item;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_conditional_break() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (true) {
    if (done) {
        break;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_normal_loop() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (item in items) {
    process(item);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_empty_loop() {
        let test = TestProgram::for_rule_without_builtins(NoLoopSingleIteration);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (getNext()) {
    // something
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }
}
