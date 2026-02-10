use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignment operators in return statements.
    ///
    /// Assignments in return statements are often mistakes where `=` was typed
    /// instead of `==`. If intentional, separate the assignment from the return.
    #[lint(
        id = "no-return-assign",
        code = "LU027",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoReturnAssign,
    "Disallow assignment in return statements"
}

impl LintRule for NoReturnAssign {
    fn meta(&self) -> &'static crate::LintMeta {
        NoReturnAssign::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Return {
                value: Some(value_id),
            } = expression
            else {
                continue;
            };

            // check if the return value is an assignment
            if contains_assignment(ctx, *value_id) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    NO_RETURN_ASSIGN.id,
                    NO_RETURN_ASSIGN.code,
                    NO_RETURN_ASSIGN.category,
                    severity,
                    "assignment in return statement",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("separate assignment from return");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = no_return_assign_fix(ctx, node_id, *value_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return whether the expression contains an assignment.
fn contains_assignment(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expr_id);
    match expression {
        ast::Expression::Assign { .. } => true,
        ast::Expression::Parenthesized { expression } => contains_assignment(ctx, *expression),
        _ => false,
    }
}

/// Return one assignment expression id, unwrapping parentheses.
fn assignment_expression_id(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let expression = ctx.tree.get(expr_id);
    match expression {
        ast::Expression::Assign { .. } => Some(expr_id),
        ast::Expression::Parenthesized { expression } => assignment_expression_id(ctx, *expression),
        _ => None,
    }
}

/// Build an unsafe fix that lifts assignment out of return position.
fn no_return_assign_fix(
    ctx: &LintModuleAstContext<'_>,
    return_id: ast::LocalNodeId<ast::Expression>,
    value_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let assignment_id = assignment_expression_id(ctx, value_id)?;
    let assignment_span = ctx.tree.get_span(assignment_id);
    let assignment_text = ctx.get_span_text(assignment_span);
    if assignment_text.trim().is_empty() {
        return None;
    }

    let binding_name = unique_binding_name(ctx, "__destackReturnAssignValue");
    let replacement =
        format!("{{ const {binding_name} = ({assignment_text}); return {binding_name}; }}");
    let return_span = ctx.tree.get_span(return_id);
    let edits = ctx
        .edit_builder()
        .replace(return_span, replacement)
        .into_edits();

    Some(LintFix::r#unsafe("Move assignment out of return").with_edits(edits))
}

/// Build a unique binding name not present in the current source file.
fn unique_binding_name(ctx: &LintModuleAstContext<'_>, base_name: &str) -> String {
    if !ctx.file.text().contains(base_name) {
        return base_name.to_string();
    }

    let mut index = 1_u32;
    loop {
        let candidate = format!("{base_name}{index}");
        if !ctx.file.text().contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_return_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_detects_return_assignment.ts",
            "function foo() { return x = 1; }",
        );
        test.result(result)
            .assert_lint("no-return-assign")
            .assert_has_fix("no-return-assign");
    }

    #[test]
    fn test_detects_parenthesized_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_detects_parenthesized_assignment.ts",
            "function foo() { return (x = 1); }",
        );
        test.result(result).assert_lint("no-return-assign");
    }

    #[test]
    fn test_allows_normal_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_allows_normal_return.ts",
            "function foo() { return x; }",
        );
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_allows_comparison_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_allows_comparison_in_return.ts",
            "function foo() { return x == 1; }",
        );
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_allows_empty_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_allows_empty_return.ts",
            "function foo() { return; }",
        );
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_fix_rewrites_return_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_fix_rewrites_return_assignment.ts",
            r#"
function foo() {
    return x = 1
}
"#,
        );
        test.result(result)
            .assert_lint("no-return-assign")
            .assert_unsafe_fixed(
                r#"
function foo() {
    {
        const __destackReturnAssignValue = (x = 1);
        return __destackReturnAssignValue;
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_uses_unique_binding_name_on_collision() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_fix_uses_unique_binding_name_on_collision.ts",
            r#"
const __destackReturnAssignValue = 0

function foo() {
    return x = 1
}
"#,
        );
        test.result(result)
            .assert_lint("no-return-assign")
            .assert_unsafe_fixed(
                r#"
const __destackReturnAssignValue = 0;

function foo() {
    {
        const __destackReturnAssignValue1 = (x = 1);
        return __destackReturnAssignValue1;
    }
}
"#,
            );
    }

    #[test]
    fn test_mutation_detects_compound_assignment_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint_ast(
            "no_return_assign/test_mutation_detects_compound_assignment_return.ts",
            r#"
function foo() {
    return total += step
}
"#,
        );
        test.result(result)
            .assert_lint("no-return-assign")
            .assert_unsafe_fixed(
                r#"
function foo() {
    {
        const __destackReturnAssignValue = (total += step);
        return __destackReturnAssignValue;
    }
}
"#,
            );
    }
}
