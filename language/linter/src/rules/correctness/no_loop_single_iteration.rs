use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow loops that execute at most once.
    ///
    /// A loop that always exits on the first iteration is likely a bug.
    /// This happens when the loop body unconditionally contains a
    /// return, break, or throw statement.
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
        code = "LC022",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoLoopSingleIteration,
    "Disallow loops that execute at most once"
}

impl LintRule for NoLoopSingleIteration {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoLoopSingleIteration::meta()
    }

    /// Check module AST nodes for loops that always exit after one iteration.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
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

                // build diagnostic payload
                let mut diagnostic = LintReport::new(
                    NO_LOOP_SINGLE_ITERATION.id,
                    NO_LOOP_SINGLE_ITERATION.code,
                    NO_LOOP_SINGLE_ITERATION.category,
                    severity,
                    format!("{loop_type} loop executes at most once"),
                    ctx.tree.get_span(node_id),
                )
                .label("body unconditionally exits on first iteration");

                // rewrite trivial `loop { return ... }` and `loop { throw ... }` forms
                if ctx.compute_fixes
                    && let Some(fix) = no_loop_single_iteration_fix(ctx, node_id, expression)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build one unsafe fix for trivial `loop` forms that immediately return or throw.
fn no_loop_single_iteration_fix(
    ctx: &LintAstContext<'_>,
    loop_expression_id: ast::LocalNodeId<ast::Expression>,
    loop_expression: &ast::Expression,
) -> Option<LintFix> {
    // keep bare loop forms only: other loop kinds may carry setup side effects
    let ast::Expression::Loop { body, .. } = loop_expression else {
        return None;
    };

    // resolve block
    let block = ctx.tree.get(*body);
    if block.len() != 1 {
        return None;
    }

    // resolve single expression id
    let single_expression_id = block.first_expression().unwrap();
    let single_expression = unwrap_statement_expression(ctx, single_expression_id);
    if !matches!(
        single_expression,
        ast::Expression::Return { .. } | ast::Expression::Throw { .. }
    ) {
        return None;
    }

    // build replacement text
    let replacement_text = ctx
        .get_span_text(ctx.tree.get_span(single_expression_id))
        .to_string();
    if replacement_text.trim().is_empty() {
        return None;
    }

    // replace the loop with the single control flow statement
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(loop_expression_id), replacement_text)
        .into_edits();
    Some(LintFix::r#unsafe("Replace one-shot loop with direct control flow").with_edits(edits))
}

/// Check if a block unconditionally exits (return, break, throw, continue).
fn body_unconditionally_exits(
    ctx: &LintAstContext<'_>,
    body_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    // compute flow outcomes for the loop body
    let flow = block_flow(ctx, body_id);
    let allows_second_iteration = flow.reaches_next_statement || flow.reaches_next_iteration;

    !allows_second_iteration
}

/// Flow outcomes for one expression subtree.
#[derive(Clone, Copy, Debug, Default)]
struct LoopFlow {
    /// Whether one path reaches the next statement in the same block.
    reaches_next_statement: bool,
    /// Whether one path reaches the next iteration directly (for example, `continue`).
    reaches_next_iteration: bool,
}

/// Return flow for expressions that always exit the current iteration.
const fn flow_exit_iteration() -> LoopFlow {
    LoopFlow {
        reaches_next_statement: false,
        reaches_next_iteration: false,
    }
}

/// Return flow for expressions that fall through to the next statement.
const fn flow_fallthrough() -> LoopFlow {
    LoopFlow {
        reaches_next_statement: true,
        reaches_next_iteration: false,
    }
}

/// Return flow for expressions that continue the current loop.
const fn flow_continue_iteration() -> LoopFlow {
    LoopFlow {
        reaches_next_statement: false,
        reaches_next_iteration: true,
    }
}

/// Evaluate block flow outcomes.
fn block_flow(ctx: &LintAstContext<'_>, body_id: ast::LocalNodeId<ast::Block>) -> LoopFlow {
    let block = ctx.tree.get(body_id);
    let mut reaches_next_statement = true;
    let mut reaches_next_iteration = false;

    // evaluate each expression in order while statement flow is still reachable
    for expression_id in block.iter_expressions() {
        if !reaches_next_statement {
            break;
        }

        // fold the next expression flow into the block state
        let flow = expression_flow(ctx, expression_id);
        reaches_next_iteration = reaches_next_iteration || flow.reaches_next_iteration;
        reaches_next_statement = flow.reaches_next_statement;
    }

    LoopFlow {
        reaches_next_statement,
        reaches_next_iteration,
    }
}

/// Evaluate flow outcomes for one expression.
fn expression_flow(
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> LoopFlow {
    let expr = ctx.tree.get(expr_id);

    // branch by expression kind to model control flow
    match expr {
        // direct exit statements
        ast::Expression::Return { .. } => flow_exit_iteration(),
        ast::Expression::Break { .. } => flow_exit_iteration(),
        ast::Expression::Throw { .. } => flow_exit_iteration(),

        // continue reaches the loop's next iteration
        ast::Expression::Continue { .. } => flow_continue_iteration(),

        // block: evaluate statement order and flow
        ast::Expression::Block(block_id) => block_flow(ctx, *block_id),

        // if/else: merge both branch outcomes
        ast::Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            let then_flow = expression_flow(ctx, *then_expression);
            let else_flow = else_expression
                .map(|expression_id| expression_flow(ctx, expression_id))
                .unwrap_or_else(flow_fallthrough);

            LoopFlow {
                reaches_next_statement: then_flow.reaches_next_statement
                    || else_flow.reaches_next_statement,
                reaches_next_iteration: then_flow.reaches_next_iteration
                    || else_flow.reaches_next_iteration,
            }
        }

        // complex control flow: conservatively allow fallthrough
        ast::Expression::Match { .. } | ast::Expression::Try { .. } => flow_fallthrough(),

        // other expressions may fall through
        _ => flow_fallthrough(),
    }
}

/// Unwrap one statement wrapper and return the underlying expression.
fn unwrap_statement_expression<'a>(
    ctx: &'a LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> &'a ast::Expression {
    ctx.tree.get(expression_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_for_with_unconditional_return() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_detects_for_with_unconditional_return.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_detects_foreach_with_unconditional_return.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_detects_while_with_unconditional_break.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_detects_loop_with_unconditional_throw.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_conditional_return.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_conditional_break.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_normal_loop.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_empty_loop.ds",
            r#"
while (getNext()) {
    // something
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_fix_rewrites_trivial_loop_return() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_fix_rewrites_trivial_loop_return.ds",
            r#"
loop {
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-loop-single-iteration")
            .assert_unsafe_fixed(
                r#"
return value;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_for_loop_with_return() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_no_fix_for_for_loop_with_return.ds",
            r#"
for (let i = 0; i < 1; i++) {
    return i;
}
"#,
        );
        test.result(result)
            .assert_lint("no-loop-single-iteration")
            .assert_has_no_fix("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_loop_with_unconditional_continue() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_loop_with_unconditional_continue.ds",
            r#"
while (ready) {
    continue;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_for_loop_with_unconditional_continue() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_for_loop_with_unconditional_continue.ds",
            r#"
for (let i = 0; i < 10; i++) {
    continue;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_allows_continue_branch_with_return_fallback() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_allows_continue_branch_with_return_fallback.ds",
            r#"
while (true) {
    if (shouldContinue) {
        continue;
    }
    return value;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-loop-single-iteration");
    }

    #[test]
    fn test_detects_if_else_all_paths_exit() {
        let test = TestProgram::for_rule_without_prelude(NoLoopSingleIteration);
        let result = test.lint_ast(
            "no_loop_single_iteration/test_detects_if_else_all_paths_exit.ds",
            r#"
for (item in items) {
    if (item.done) {
        break;
    } else {
        return item;
    }
}
"#,
        );
        test.result(result).assert_lint("no-loop-single-iteration");
    }
}
