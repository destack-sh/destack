use destack_dir as dir;
use destack_repository::config::{ConditionAssignmentMode, LintSeverity};

use crate::rules::common::{
    ConditionAssignmentStyle, condition_assignment_style, control_flow_condition_expression,
    expression_contains_assignment,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignment expressions in conditional statements.
    ///
    /// Using an assignment in a condition is often a mistake: `if (x = 1)` was
    /// probably meant to be `if (x == 1)`. If assignment is intentional, wrap
    /// it in extra parentheses: `if (((x = getValue())))`.
    #[lint(
        id = "no-cond-assign",
        code = "LU003",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoCondAssign,
    "Disallow assignment in conditions"
}

impl LintRule for NoCondAssign {
    fn meta(&self) -> &'static LintMeta {
        NoCondAssign::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect conditional expression nodes
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            // resolve the condition expression for supported control flow forms
            let expression = ctx.dir.get(node_id);
            let Some(condition_id) = control_flow_condition_expression(expression) else {
                continue;
            };

            // classify assignment wrapping style in the condition
            let assignment_style = condition_assignment_style(ctx.dir.tree(), condition_id);
            let has_assignment = expression_contains_assignment(ctx.dir.tree(), condition_id);
            match ctx.options().correctness.no_cond_assign_mode {
                ConditionAssignmentMode::ExceptParens
                    if assignment_style == ConditionAssignmentStyle::None =>
                {
                    continue;
                }
                ConditionAssignmentMode::Always if !has_assignment => {
                    continue;
                }
                _ => {}
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let condition_span = ctx.dir.get_span(condition_id);
            let mut diagnostic = LintReport::new(
                NO_COND_ASSIGN.id,
                NO_COND_ASSIGN.code,
                NO_COND_ASSIGN.category,
                severity,
                "assignment in condition",
                condition_span,
            )
            .label("did you mean `==`?");

            // only the except-parens mode has an intent preserving wrap fix
            if ctx.options().correctness.no_cond_assign_mode
                == ConditionAssignmentMode::ExceptParens
                && ctx.compute_fixes
            {
                let condition_text = ctx.get_span_text(condition_span);
                let replacement = match assignment_style {
                    ConditionAssignmentStyle::Bare => format!("(({condition_text}))"),
                    ConditionAssignmentStyle::SingleParenthesized => {
                        format!("({condition_text})")
                    }
                    ConditionAssignmentStyle::None => String::new(),
                };
                if !replacement.is_empty() {
                    let edits = ctx
                        .edit_builder()
                        .replace(condition_span, replacement)
                        .into_edits();
                    let fix = LintFix::safe("Wrap assignment in explicit extra parentheses")
                        .with_edits(edits);
                    diagnostic = diagnostic.fix(fix);
                }
            }

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_if_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_detects_if_assignment.ds",
            r#"
if (x = 1) {
    console.log(x);
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_has_fix("no-cond-assign");
    }

    #[test]
    fn test_detects_while_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_detects_while_assignment.ds",
            r#"
while (x = getValue()) {
    process(x);
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_has_fix("no-cond-assign");
    }

    #[test]
    fn test_detects_for_condition_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_detects_for_condition_assignment.ds",
            r#"
for (; x = next(); ) {
    process(x);
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_has_fix("no-cond-assign");
    }

    #[test]
    fn test_allows_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_allows_comparison.ds",
            r#"
if (x == 1) {
    console.log(x);
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_strict_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_allows_strict_comparison.ds",
            r#"
if (x === 1) {
    console.log(x);
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_allows_boolean_condition.ds",
            r#"
if (isReady) {
    start();
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_function_call_condition() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_allows_function_call_condition.ds",
            r#"
while (hasMore()) {
    processNext()
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_let_expression_in_condition() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_allows_let_expression_in_condition.ds",
            r#"
if (const x = getValue()) {
    process(x)
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_allows_double_parenthesized_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_allows_double_parenthesized_assignment.ds",
            r#"
if (((x = getValue()))) {
    process(x)
}
"#,
        );
        test.result(result).assert_no_lint("no-cond-assign");
    }

    #[test]
    fn test_fix_bare_assignment_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_fix_bare_assignment_in_if.ds",
            r#"
if (x = 1) {
    process(x)
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_safe_fixed(
                r#"
if (((x = 1))) {
    process(x)
}
"#,
            );
    }

    #[test]
    fn test_fix_single_parenthesized_assignment_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_fix_single_parenthesized_assignment_in_if.ds",
            r#"
if ((x = 1)) {
    process(x)
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_safe_fixed(
                r#"
if (((x = 1))) {
    process(x)
}
"#,
            );
    }

    #[test]
    fn test_fix_bare_assignment_in_for_condition() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_fix_bare_assignment_in_for_condition.ds",
            r#"for (; x = next(); ) {}"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_safe_fixed(
                r#"
for (; ((x = next())); ) {}
"#,
            );
    }

    #[test]
    fn test_mutation_detects_assignments_in_if_and_while() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint(
            "no_cond_assign/test_mutation_detects_assignments_in_if_and_while.ds",
            r#"
if (first = read()) {
    process(first)
}
while (next = read()) {
    process(next)
}
"#,
        );
        test.result(result).assert_lint_count("no-cond-assign", 2);
    }

    #[test]
    fn test_detects_nested_assignment_when_always_mode_is_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign).with_options(|options| {
            options.correctness.no_cond_assign_mode = ConditionAssignmentMode::Always;
        });
        let result = test.lint(
            "no_cond_assign/test_detects_nested_assignment_when_always_mode_is_enabled.ds",
            r#"
if (isReady || (next = read())) {
    process(next)
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_has_no_fix("no-cond-assign");
    }

    #[test]
    fn test_detects_double_parenthesized_assignment_when_always_mode_is_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign).with_options(|options| {
            options.correctness.no_cond_assign_mode = ConditionAssignmentMode::Always;
        });
        let result = test.lint(
            "no_cond_assign/test_detects_double_parenthesized_assignment_when_always_mode_is_enabled.ds",
            r#"
if (((next = read()))) {
    process(next)
}
"#,
        );
        test.result(result)
            .assert_lint("no-cond-assign")
            .assert_has_no_fix("no-cond-assign");
    }
}
