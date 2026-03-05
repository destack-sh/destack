use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

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
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // inspect conditional expression nodes
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // resolve the condition expression for supported control flow forms
            let condition_id = match ctx.tree.get(node_id) {
                ast::Expression::If { condition, .. } => match condition {
                    ast::IfCondition::Expression { condition } => *condition,
                    ast::IfCondition::Let { .. } => continue,
                },
                ast::Expression::While { condition, .. } => *condition,
                ast::Expression::For {
                    condition: Some(condition),
                    ..
                } => *condition,
                _ => continue,
            };

            // classify assignment wrapping style in the condition
            let assignment_style = assignment_style(ctx, condition_id);
            if assignment_style == AssignmentStyle::None {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the explicit parenthesized replacement
            let condition_span = ctx.tree.get_span(condition_id);
            let condition_text = ctx.get_span_text(condition_span);
            let replacement = match assignment_style {
                AssignmentStyle::Bare => format!("(({condition_text}))"),
                AssignmentStyle::SingleParenthesized => format!("({condition_text})"),
                AssignmentStyle::None => unreachable!(),
            };
            let edits = ctx
                .edit_builder()
                .replace(condition_span, replacement)
                .into_edits();
            let fix =
                LintFix::safe("Wrap assignment in explicit extra parentheses").with_edits(edits);

            // report the assignment style diagnostic with an intent preserving fix
            ctx.report(
                LintDiagnostic::new(
                    NO_COND_ASSIGN.id,
                    NO_COND_ASSIGN.code,
                    NO_COND_ASSIGN.category,
                    severity,
                    "assignment in condition",
                    ctx.module.file_id,
                    condition_span,
                )
                .with_label("did you mean `==`?")
                .with_fix(fix),
            );
        }
    }
}

/// The assignment wrapping style in one condition expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssignmentStyle {
    /// No direct assignment-like matched.
    None,
    /// Bare assignment expression: `x = y`.
    Bare,
    /// Single parenthesized assignment: `(x = y)`.
    SingleParenthesized,
}

/// Return the assignment wrapping style for one condition expression.
/// Let expressions are allowed (like Rust's `if let`).
fn assignment_style(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> AssignmentStyle {
    // inspect the outer condition expression
    let expr = ctx.tree.get(expr_id);
    match expr {
        // let expressions are allowed (like `if const Some(x) = foo()`)
        ast::Expression::Let { .. } | ast::Expression::Using { .. } => AssignmentStyle::None,
        ast::Expression::Assign { .. } => AssignmentStyle::Bare,
        ast::Expression::Parenthesized { expression } => {
            // detect one explicit parenthesized assignment
            let inner = ctx.tree.get(*expression);
            if matches!(inner, ast::Expression::Assign { .. }) {
                AssignmentStyle::SingleParenthesized
            } else {
                AssignmentStyle::None
            }
        }
        _ => AssignmentStyle::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_if_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoCondAssign);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
}
