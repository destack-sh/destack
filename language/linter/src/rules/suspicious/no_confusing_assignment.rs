use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    ConditionAssignmentStyle, condition_assignment_style, control_flow_condition_expression,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Warn on assignments that look like comparisons.
    ///
    /// An assignment in a context where a comparison is expected (like `if (x = 1)`)
    /// is often a mistake. Use `===` for comparison or wrap in extra parentheses
    /// if assignment is intentional.
    #[lint(
        id = "no-confusing-assignment",
        code = "LU004",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoConfusingAssignment,
    "Warn on assignments that look like comparisons"
}

impl LintRule for NoConfusingAssignment {
    fn meta(&self) -> &'static LintMeta {
        NoConfusingAssignment::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check conditions in if/while/for that contain assignments
            let Some(condition_id) = control_flow_condition_expression(expression) else {
                continue;
            };

            // check if condition is an assignment (not wrapped in extra parens)
            let assignment_style = condition_assignment_style(ctx.tree, condition_id);
            if assignment_style != ConditionAssignmentStyle::None {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // add explicit grouping fix to silence confusing assignment intent
                let condition_span = ctx.tree.get_span(condition_id);
                let condition_text = ctx.get_span_text(condition_span);
                let replacement = match assignment_style {
                    ConditionAssignmentStyle::Bare => format!("(({condition_text}))"),
                    ConditionAssignmentStyle::SingleParenthesized => format!("({condition_text})"),
                    ConditionAssignmentStyle::None => unreachable!(),
                };
                let edits = ctx
                    .edit_builder()
                    .replace(condition_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Wrap assignment in explicit extra parentheses")
                    .with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_CONFUSING_ASSIGNMENT.id,
                        NO_CONFUSING_ASSIGNMENT.code,
                        NO_CONFUSING_ASSIGNMENT.category,
                        severity,
                        "assignment in condition",
                        ctx.module.file_id,
                        condition_span,
                    )
                    .with_label("use `===` for comparison or wrap assignment in extra parentheses")
                    .with_fix(fix),
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
    fn test_detects_assignment_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_detects_assignment_in_if.ts",
            "if (x = 1) {}",
        );
        test.result(result)
            .assert_lint("no-confusing-assignment")
            .assert_has_fix("no-confusing-assignment");
    }

    #[test]
    fn test_detects_assignment_in_while() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_detects_assignment_in_while.ts",
            "while (x = next()) {}",
        );
        test.result(result)
            .assert_lint("no-confusing-assignment")
            .assert_has_fix("no-confusing-assignment");
    }

    #[test]
    fn test_detects_single_paren_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_detects_single_paren_assignment.ts",
            "if ((x = 1)) {}",
        );
        test.result(result)
            .assert_lint("no-confusing-assignment")
            .assert_has_fix("no-confusing-assignment");
    }

    #[test]
    fn test_allows_double_paren_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_allows_double_paren_assignment.ts",
            "if (((x = 1))) {}",
        );
        test.result(result)
            .assert_no_lint("no-confusing-assignment");
    }

    #[test]
    fn test_allows_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_allows_comparison.ts",
            "if (x === 1) {}",
        );
        test.result(result)
            .assert_no_lint("no-confusing-assignment");
    }

    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_allows_boolean_condition.ts",
            "if (x) {}",
        );
        test.result(result)
            .assert_no_lint("no-confusing-assignment");
    }

    #[test]
    fn test_fix_bare_assignment_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_fix_bare_assignment_in_if.ts",
            "if (x = 1) {}",
        );
        test.result(result)
            .assert_lint("no-confusing-assignment")
            .assert_safe_fixed(
                r#"
if (((x = 1))) {
}
"#,
            );
    }

    #[test]
    fn test_fix_single_parenthesized_assignment_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_fix_single_parenthesized_assignment_in_if.ts",
            "if ((x = 1)) {}",
        );
        test.result(result)
            .assert_lint("no-confusing-assignment")
            .assert_safe_fixed(
                r#"
if (((x = 1))) {
}
"#,
            );
    }

    #[test]
    fn test_fix_assignment_in_for_condition() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_fix_assignment_in_for_condition.ts",
            "for (; x = next(); ) {}",
        );
        test.result(result)
            .assert_lint("no-confusing-assignment")
            .assert_safe_fixed(
                r#"
for (; ((x = next())); ) {}
"#,
            );
    }

    #[test]
    fn test_mutation_detects_different_assignment_values() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingAssignment);
        let result = test.lint_ast(
            "no_confusing_assignment/test_mutation_detects_different_assignment_values.ts",
            r#"
if (flag = compute()) {}
while (ready = check()) {}
"#,
        );
        test.result(result)
            .assert_lint_count("no-confusing-assignment", 2);
    }
}
