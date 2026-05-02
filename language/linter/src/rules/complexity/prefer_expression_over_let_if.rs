use crate::LintMeta;
use destack_ast::{self as ast, Pattern};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_is_unqualified_path_name, expression_unwrap_statement_source_form,
};
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer direct expressions over let-if sequences.
    ///
    /// Instead of declaring a variable and then assigning in branches, use an if expression or ternary to initialize directly.
    #[lint(
        id = "prefer-expression-over-let-if",
        code = "LX024",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferExpressionOverLetIf,
    "Prefer expression over let-if"
}

/// Get the binding name from a pattern if it's a simple identifier.
fn get_binding_name(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<Pattern>,
) -> Option<destack_core::StringId> {
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
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    target_name: destack_core::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Assign { left, .. } => {
            // check if left is the same variable
            if assign_pattern_is_unqualified_path_name(ctx.tree, *left, target_name) {
                return true;
            }

            false
        }
        _ => false,
    }
}

/// Check if an expression (which should be a block) contains only an assignment to the target.
fn expr_is_simple_assignment(
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    target_name: destack_core::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);

    // if it's a block expression, check its contents
    if let ast::Expression::Block(block_id) = expr {
        let block = ctx.tree.get(*block_id);
        if block.len() == 1 {
            return is_assignment_to(ctx, block.first_expression().unwrap(), target_name);
        }
    }

    false
}

impl LintRule for PreferExpressionOverLetIf {
    fn meta(&self) -> &'static LintMeta {
        PreferExpressionOverLetIf::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // look for blocks with potential let-if patterns
        for block_id in ctx.tree.iter_nodes::<ast::Block>() {
            let block = ctx.tree.get(block_id);
            let expression_ids = block.iter_expressions().collect::<Vec<_>>();

            // need at least 2 expressions
            if expression_ids.len() < 2 {
                continue;
            }

            // check consecutive pairs
            for i in 0..expression_ids.len() - 1 {
                let first_id = expression_ids[i];
                let second_id = expression_ids[i + 1];

                // unwrap statement wrappers
                let let_expression_id = expression_unwrap_statement_source_form(ctx.tree, first_id);
                let if_expression_id = expression_unwrap_statement_source_form(ctx.tree, second_id);
                let first_expr = ctx.tree.get(let_expression_id);
                let second_expr = ctx.tree.get(if_expression_id);

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
                    condition,
                    else_expression: Some(else_expr),
                    ..
                } = second_expr
                else {
                    continue;
                };
                let ast::IfCondition::Expression { condition } = condition else {
                    continue;
                };

                // both branches must be simple assignments to the variable
                if !expr_is_simple_assignment(ctx, *then_expression, var_name) {
                    continue;
                }
                if !expr_is_simple_assignment(ctx, *else_expr, var_name) {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, first_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintReport::new(
                    PREFER_EXPRESSION_OVER_LET_IF.id,
                    PREFER_EXPRESSION_OVER_LET_IF.code,
                    PREFER_EXPRESSION_OVER_LET_IF.category,
                    severity,
                    "prefer direct expression over let-if sequence",
                    ctx.tree.get_span(first_id),
                )
                .label("use if expression to initialize directly");
                if ctx.compute_fixes
                    && let Some(fix) = prefer_expression_over_let_if_fix(
                        ctx,
                        let_expression_id,
                        if_expression_id,
                        *condition,
                        *then_expression,
                        *else_expr,
                        var_name,
                    )
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build one suggestion fix by rewriting let-if assignment chains to one if-expression init.
#[allow(clippy::too_many_arguments)]
fn prefer_expression_over_let_if_fix(
    ctx: &LintAstContext<'_>,
    let_expression_id: ast::LocalNodeId<ast::Expression>,
    if_expression_id: ast::LocalNodeId<ast::Expression>,
    condition_expression_id: ast::LocalNodeId<ast::Expression>,
    then_expression_id: ast::LocalNodeId<ast::Expression>,
    else_expression_id: ast::LocalNodeId<ast::Expression>,
    target_name: destack_core::StringId,
) -> Option<LintFix> {
    let let_text = ctx
        .get_span_text(ctx.tree.get_span(let_expression_id))
        .to_string();
    if let_text.trim().is_empty() {
        return None;
    }

    let condition_text = ctx
        .get_span_text(ctx.tree.get_span(condition_expression_id))
        .to_string();
    if condition_text.trim().is_empty() {
        return None;
    }

    let then_value_text = assignment_value_text(ctx, then_expression_id, target_name)?;
    let else_value_text = assignment_value_text(ctx, else_expression_id, target_name)?;

    let replacement_text = format!(
        "{let_text} = if {condition_text} {{\n    {then_value_text}\n}} else {{\n    {else_value_text}\n}}"
    );

    let let_span = ctx.tree.get_span(let_expression_id);
    let if_span = ctx.tree.get_span(if_expression_id);
    if if_span.start <= let_span.start {
        return None;
    }

    let span = destack_source::Span::new(ctx.file_id(), let_span.start, if_span.end);
    let edits = ctx
        .edit_builder()
        .replace(span, replacement_text)
        .into_edits();
    Some(
        LintFix::suggestion("Rewrite let-if chain as direct if-expression initialization")
            .with_edits(edits),
    )
}

/// Return the assignment right-hand side text for one branch expression.
fn assignment_value_text(
    ctx: &LintAstContext<'_>,
    branch_expression_id: ast::LocalNodeId<ast::Expression>,
    target_name: destack_core::StringId,
) -> Option<String> {
    let branch_expression = ctx.tree.get(branch_expression_id);
    let ast::Expression::Block(block_id) = branch_expression else {
        return None;
    };
    let block = ctx.tree.get(*block_id);
    if block.len() != 1 {
        return None;
    }

    let branch_statement_id = block.first_expression().unwrap();
    let assignment_expression_id =
        expression_unwrap_statement_source_form(ctx.tree, branch_statement_id);
    let assignment_expression = ctx.tree.get(assignment_expression_id);
    let ast::Expression::Assign { left, right, .. } = assignment_expression else {
        return None;
    };
    if !assign_pattern_is_unqualified_path_name(ctx.tree, *left, target_name) {
        return None;
    }

    let right_text = ctx.get_span_text(ctx.tree.get_span(*right)).to_string();
    if right_text.trim().is_empty() {
        return None;
    }

    Some(right_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_let_if_sequence_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "prefer_expression_over_let_if/test_let_if_sequence_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "prefer_expression_over_let_if/test_direct_expression_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "prefer_expression_over_let_if/test_let_with_initializer_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "prefer_expression_over_let_if/test_if_without_else_allowed.ds",
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

    #[test]
    fn test_fix_rewrites_let_if_chain_to_if_expression_init() {
        let test = TestProgram::for_rule_without_prelude(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "prefer_expression_over_let_if/test_fix_rewrites_let_if_chain_to_if_expression_init.ds",
            r#"
function run(flag: bool) {
    let value: int32
    if (flag) {
        value = 1
    } else {
        value = 2
    }
}
"#,
        );
        test.result(result.clone())
            .assert_lint("prefer-expression-over-let-if")
            .assert_has_fix("prefer-expression-over-let-if");

        let fixed = test.result(result).apply_fixes(None);
        assert_eq!(
            fixed.trim(),
            r#"
function run(flag: bool) {
    let value: int32 = if (flag) {
        1
    } else {
        2
    };
}
"#
            .trim()
        );
    }

    #[test]
    fn test_no_fix_when_if_branch_contains_multiple_statements() {
        let test = TestProgram::for_rule_without_prelude(PreferExpressionOverLetIf);
        let result = test.lint_ast(
            "prefer_expression_over_let_if/test_no_fix_when_if_branch_contains_multiple_statements.ds",
            r#"
function run(flag: bool) {
    let value: int32
    if (flag) {
        value = 1
        log(value)
    } else {
        value = 2
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-expression-over-let-if");
    }
}
