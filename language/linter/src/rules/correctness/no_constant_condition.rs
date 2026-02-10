use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow constant expressions in conditions.
    ///
    /// Using a constant expression like `if (true)` or `while (false)` is
    /// usually a mistake. If intentional, consider restructuring the code.
    #[lint(
        id = "no-constant-condition",
        code = "LC009",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoConstantCondition,
    "Disallow constant expressions in conditions"
}

impl LintRule for NoConstantCondition {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConstantCondition::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let condition_id = match ctx.tree.get(node_id) {
                ast::Expression::If { condition, .. } => match condition {
                    ast::IfCondition::Expression { condition } => *condition,
                    ast::IfCondition::Let { .. } => continue,
                },
                ast::Expression::While { condition, .. } => *condition,
                _ => continue,
            };
            if ctx.const_value(condition_id).is_none() {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                NO_CONSTANT_CONDITION.id,
                NO_CONSTANT_CONDITION.code,
                NO_CONSTANT_CONDITION.category,
                severity,
                "unexpected constant condition",
                ctx.module.file_id,
                ctx.tree.get_span(condition_id),
            )
            .with_label("this condition is always the same");
            if ctx.compute_fixes
                && let Some(fix) = no_constant_condition_fix(ctx, node_id)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a conservative fix for constant conditions.
fn no_constant_condition_fix(
    ctx: &mut LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let expression = ctx.tree.get(expression_id);

    // simplify constant if expressions
    if let ast::Expression::If {
        condition,
        then_expression,
        else_expression,
        ..
    } = expression
    {
        let ast::IfCondition::Expression { condition } = condition else {
            return None;
        };
        let condition_is_true = ctx.const_bool(*condition)?;

        // keep the selected branch text
        if condition_is_true {
            let replacement = ctx.get_span_text(ctx.tree.get_span(*then_expression));
            let edits = ctx
                .edit_builder()
                .replace(ctx.tree.get_span(expression_id), replacement)
                .into_edits();
            return Some(LintFix::safe("Inline always-true condition branch").with_edits(edits));
        }

        // use else branch when present
        if let Some(else_expression_id) = else_expression {
            let replacement = ctx.get_span_text(ctx.tree.get_span(*else_expression_id));
            let edits = ctx
                .edit_builder()
                .replace(ctx.tree.get_span(expression_id), replacement)
                .into_edits();
            return Some(LintFix::safe("Inline always-false else branch").with_edits(edits));
        }

        // otherwise delete only when in statement position
        let statement_span = parent_statement_span(ctx, expression_id)?;
        let edits = ctx.edit_builder().delete(statement_span).into_edits();
        return Some(
            LintFix::safe("Remove always-false condition statement branch").with_edits(edits),
        );
    }

    // remove `while (false)` loops in statement position
    if let ast::Expression::While {
        kind: ast::WhileKind::While,
        condition,
        ..
    } = expression
    {
        if !ctx.const_bool(*condition)? {
            let statement_span = parent_statement_span(ctx, expression_id)?;
            let edits = ctx.edit_builder().delete(statement_span).into_edits();
            return Some(LintFix::safe("Remove while loop that never executes").with_edits(edits));
        }
    }

    None
}

/// Return the parent statement wrapper span for one expression.
fn parent_statement_span(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<destack_source::Span> {
    let parent_id = ctx.parents.get(expression_id)?;
    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return None;
    }

    let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent_expression = ctx.tree.get(parent_expression_id);
    if !matches!(parent_expression, ast::Expression::Statement(_)) {
        return None;
    }

    Some(ctx.tree.get_span(parent_expression_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_if_true() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_detects_if_true.ds",
            r#"
if (true) { foo(); }
"#,
        );
        test.result(result)
            .assert_lint("no-constant-condition")
            .assert_safe_fixed(
                r#"
{
    foo();
}
"#,
            );
    }

    #[test]
    fn test_detects_if_false() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_detects_if_false.ds",
            r#"
if (false) { foo(); }
"#,
        );
        test.result(result)
            .assert_lint("no-constant-condition")
            .assert_has_no_fix("no-constant-condition");
    }

    #[test]
    fn test_detects_while_true() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_detects_while_true.ds",
            r#"
while (true) { foo(); }
"#,
        );
        test.result(result)
            .assert_lint("no-constant-condition")
            .assert_has_no_fix("no-constant-condition");
    }

    #[test]
    fn test_detects_if_number() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_detects_if_number.ds",
            r#"
if (1) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_fix_false_condition_with_else_branch() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_fix_false_condition_with_else_branch.ds",
            r#"
if (false) { foo(); } else { bar(); }
"#,
        );
        test.result(result)
            .assert_lint("no-constant-condition")
            .assert_safe_fixed(
                r#"
{
    bar();
}
"#,
            );
    }

    #[test]
    fn test_fix_while_false_removes_loop() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_fix_while_false_removes_loop.ds",
            r#"
while (false) { foo(); }
"#,
        );
        test.result(result)
            .assert_lint("no-constant-condition")
            .assert_has_no_fix("no-constant-condition");
    }

    #[test]
    fn test_no_constant_with_variable() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_no_constant_with_variable.ds",
            r#"
if (x) { foo(); }
"#,
        );
        test.result(result).assert_no_lint("no-constant-condition");
    }

    #[test]
    fn test_no_constant_with_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint_ast(
            "no_constant_condition/test_no_constant_with_comparison.ds",
            r#"
if (x > 0) { foo(); }
"#,
        );
        test.result(result).assert_no_lint("no-constant-condition");
    }
}
