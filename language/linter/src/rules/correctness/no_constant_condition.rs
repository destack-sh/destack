use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::control_flow_condition_expression;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow constant expressions in conditions.
    ///
    /// Using a constant expression like `if (true)` or `while (false)` is
    /// usually a mistake. If intentional, consider restructuring the code.
    #[lint(
        id = "no-constant-condition",
        code = "LC009",
        category = Correctness,
        level = Dir,
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConstantCondition::meta()
    }

    /// Check module source nodes for constant conditional expressions.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // walk conditional expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            // resolve the condition expression to inspect
            let condition_id = match expression {
                dir::Expression::While {
                    form, condition, ..
                } => {
                    // align source defaults: allow `while (true)` as an explicit infinite loop
                    if *form == dir::WhileForm::While && ctx.const_bool(*condition) == Some(true) {
                        continue;
                    }
                    *condition
                }
                _ => {
                    let Some(condition_id) = control_flow_condition_expression(expression) else {
                        continue;
                    };
                    condition_id
                }
            };

            // only report statically known constants
            if ctx.const_value(condition_id).is_none() {
                continue;
            }

            // resolve effective severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build diagnostic payload
            let mut diagnostic = LintReport::new(
                NO_CONSTANT_CONDITION.id,
                NO_CONSTANT_CONDITION.code,
                NO_CONSTANT_CONDITION.category,
                severity,
                "unexpected constant condition",
                ctx.dir.get_span(condition_id),
            )
            .label("this condition is always the same");

            // attach conservative autofix when available
            if ctx.compute_fixes
                && let Some(fix) = no_constant_condition_fix(ctx, node_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a conservative fix for constant conditions.
fn no_constant_condition_fix(
    ctx: &mut LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // inspect the conditional expression shape
    let expression = ctx.dir.get(expression_id);

    // simplify constant if expressions
    if let dir::Expression::If {
        condition,
        then_expression,
        else_expression,
        ..
    } = expression
    {
        let dir::IfCondition::Expression { condition } = condition else {
            return None;
        };
        let condition_is_true = ctx.const_bool(*condition)?;

        // keep the selected branch text
        if condition_is_true {
            let replacement = ctx.get_span_text(ctx.dir.get_span(*then_expression));
            let edits = ctx
                .edit_builder()
                .replace(ctx.dir.get_span(expression_id), replacement)
                .into_edits();
            return Some(LintFix::safe("Inline always-true condition branch").with_edits(edits));
        }

        // use else branch when present
        if let Some(else_expression_id) = else_expression {
            let replacement = ctx.get_span_text(ctx.dir.get_span(*else_expression_id));
            let edits = ctx
                .edit_builder()
                .replace(ctx.dir.get_span(expression_id), replacement)
                .into_edits();
            return Some(LintFix::safe("Inline always-false else branch").with_edits(edits));
        }

        return None;
    }

    // avoid deletion fixes for never-running loops
    if let dir::Expression::For {
        condition: Some(condition),
        ..
    } = expression
        && !ctx.const_bool(*condition)?
    {
        return None;
    }

    // avoid deletion fixes for never-running loops
    if let dir::Expression::While {
        form: dir::WhileForm::While,
        condition,
        ..
    } = expression
        && !ctx.const_bool(*condition)?
    {
        return None;
    }

    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_if_true() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint(
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
        let result = test.lint(
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
    fn test_allows_while_true_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint(
            "no_constant_condition/test_allows_while_true_by_default.ds",
            r#"
while (true) { foo(); }
"#,
        );
        test.result(result).assert_no_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_if_number() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_constant_condition/test_no_constant_with_comparison.ds",
            r#"
if (x > 0) { foo(); }
"#,
        );
        test.result(result).assert_no_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_do_while_true() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint(
            "no_constant_condition/test_detects_do_while_true.ds",
            r#"
do { foo(); } while (true);
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_for_true() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint(
            "no_constant_condition/test_detects_for_true.ds",
            r#"
for (; true; ) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_for_false() {
        let test = TestProgram::for_rule_without_prelude(NoConstantCondition);
        let result = test.lint(
            "no_constant_condition/test_detects_for_false.ds",
            r#"
for (; false; ) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }
}
