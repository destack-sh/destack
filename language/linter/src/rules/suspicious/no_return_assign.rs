use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{
    expression_contains_assignment, expression_subtree_mentions_identifier_name,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignment operators in return statements.
    ///
    /// Assignments in return statements are often mistakes where `=` was typed
    /// instead of `==`. If intentional, separate the assignment from the return.
    #[lint(
        id = "no-return-assign",
        code = "LU027",
        category = Suspicious,
        level = Dir,
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
    fn meta(&self) -> &'static LintMeta {
        NoReturnAssign::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let dir::Expression::Return {
                value: Some(value_id),
            } = expression
            else {
                continue;
            };

            // check if the return value is an assignment
            if !expression_contains_assignment(ctx.dir.tree(), *value_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintReport::new(
                NO_RETURN_ASSIGN.id,
                NO_RETURN_ASSIGN.code,
                NO_RETURN_ASSIGN.category,
                severity,
                "assignment in return statement",
                ctx.dir.get_span(node_id),
            )
            .label("separate assignment from return");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(fix) = no_return_assign_fix(ctx, node_id, *value_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }

        // inspect implicit return function bodies for assignment expressions
        for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(declaration_id);
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };
            let Some(body_expression_id) = declaration.body else {
                continue;
            };

            let body_expression = ctx.dir.get(body_expression_id);
            if matches!(body_expression, dir::Expression::Block(_)) {
                continue;
            }

            if !expression_contains_assignment(ctx.dir.tree(), body_expression_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, body_expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let diagnostic = LintReport::new(
                NO_RETURN_ASSIGN.id,
                NO_RETURN_ASSIGN.code,
                NO_RETURN_ASSIGN.category,
                severity,
                "assignment in implicit return expression",
                ctx.dir.get_span(body_expression_id),
            )
            .label("extract assignment before returning from this expression body");
            ctx.report(diagnostic);
        }
    }
}

/// Return one assignment expression id, unwrapping parentheses.
fn assignment_expression_id(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let expression = ctx.dir.get(expr_id);
    match expression {
        dir::Expression::Assign { .. } => Some(expr_id),
        dir::Expression::Parenthesized { expression } => assignment_expression_id(ctx, *expression),
        _ => None,
    }
}

/// Build an unsafe fix that lifts assignment out of return position.
fn no_return_assign_fix(
    ctx: &LintModuleContext<'_>,
    return_id: dir::LocalNodeId<dir::Expression>,
    value_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let assignment_id = assignment_expression_id(ctx, value_id)?;
    let assignment_span = ctx.dir.get_span(assignment_id);
    let assignment_text = ctx.get_span_text(assignment_span);
    if assignment_text.trim().is_empty() {
        return None;
    }

    let binding_name = unique_binding_name(ctx, assignment_id, "__destackReturnAssignValue");
    let replacement =
        format!("{{ const {binding_name} = ({assignment_text}); return {binding_name}; }}");
    let return_span = ctx.dir.get_span(return_id);
    let edits = ctx
        .edit_builder()
        .replace(return_span, replacement)
        .into_edits();

    Some(LintFix::r#unsafe("Move assignment out of return").with_edits(edits))
}

/// Build a unique binding name not mentioned in the rewritten assignment subtree.
fn unique_binding_name(
    ctx: &LintModuleContext<'_>,
    assignment_id: dir::LocalNodeId<dir::Expression>,
    base_name: &str,
) -> String {
    let base_name_id = ctx.string_id(base_name);
    if !expression_subtree_mentions_identifier_name(ctx.dir.tree(), assignment_id, base_name_id) {
        return base_name.to_string();
    }

    let mut index = 1_u32;
    loop {
        let candidate = format!("{base_name}{index}");
        let candidate_id = ctx.string_id(&candidate);
        if !expression_subtree_mentions_identifier_name(ctx.dir.tree(), assignment_id, candidate_id)
        {
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
        let result = test.lint(
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
        let result = test.lint(
            "no_return_assign/test_detects_parenthesized_assignment.ts",
            "function foo() { return (x = 1); }",
        );
        test.result(result).assert_lint("no-return-assign");
    }

    #[test]
    fn test_allows_normal_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_allows_normal_return.ts",
            "function foo() { return x; }",
        );
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_allows_comparison_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_allows_comparison_in_return.ts",
            "function foo() { return x == 1; }",
        );
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_allows_empty_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_allows_empty_return.ts",
            "function foo() { return; }",
        );
        test.result(result).assert_no_lint("no-return-assign");
    }

    #[test]
    fn test_fix_rewrites_return_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
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
    fn test_fix_keeps_base_binding_name_when_outer_name_is_unrelated() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_fix_keeps_base_binding_name_when_outer_name_is_unrelated.ts",
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
        const __destackReturnAssignValue = (x = 1);
        return __destackReturnAssignValue;
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_uses_unique_binding_name_when_assignment_mentions_base_name() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_fix_uses_unique_binding_name_when_assignment_mentions_base_name.ts",
            r#"
const __destackReturnAssignValue = 0

function foo() {
    return x = __destackReturnAssignValue + 1
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
        const __destackReturnAssignValue1 = (x = __destackReturnAssignValue + 1);
        return __destackReturnAssignValue1;
    }
}
"#,
            );
    }

    #[test]
    fn test_mutation_detects_compound_assignment_return() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
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

    #[test]
    fn test_detects_nested_assignment_inside_return_expression() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_detects_nested_assignment_inside_return_expression.ts",
            r#"
function foo() {
    return wrap(x = 1)
}
"#,
        );
        test.result(result).assert_lint("no-return-assign");
    }

    #[test]
    fn test_detects_assignment_in_implicit_return_function_body() {
        let test = TestProgram::for_rule_without_prelude(NoReturnAssign);
        let result = test.lint(
            "no_return_assign/test_detects_assignment_in_implicit_return_function_body.ts",
            r#"
const foo = (): int32 => x = 1
"#,
        );
        test.result(result)
            .assert_lint("no-return-assign")
            .assert_has_no_fix("no-return-assign");
    }
}
