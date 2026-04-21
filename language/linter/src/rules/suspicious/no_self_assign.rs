use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{assign_pattern_expression, assign_pattern_is_equal};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignments where both sides are the same.
    ///
    /// Assignments like `x = x` have no effect and are likely mistakes.
    #[lint(
        id = "no-self-assign",
        code = "LU028",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoSelfAssign,
    "Disallow self-assignment"
}

impl LintRule for NoSelfAssign {
    fn meta(&self) -> &'static LintMeta {
        NoSelfAssign::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect direct assignments
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            let ast::Expression::Assign {
                left,
                operator,
                right,
            } = expr
            else {
                continue;
            };
            if !matches!(
                operator,
                ast::AssignOperator::Assign
                    | ast::AssignOperator::AndAssign
                    | ast::AssignOperator::OrAssign
                    | ast::AssignOperator::CoalesceAssign
            ) {
                continue;
            }

            // keep property assignments behind the upstream option
            let checks_properties = ctx.options.correctness.no_self_assign_check_properties;
            if !checks_properties && assign_pattern_is_property_assignment_target(ctx.tree, *left) {
                continue;
            }

            // compare assignment operands structurally
            let Some(left_expression_id) = assign_pattern_expression(ctx.tree, *left) else {
                continue;
            };
            let left_span = ctx.tree.get_span(left_expression_id);
            if !assign_pattern_is_equal(ctx, *left, *right) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.tree.get_span(node_id);

            let mut diagnostic = LintDiagnostic::new(
                NO_SELF_ASSIGN.id,
                NO_SELF_ASSIGN.code,
                NO_SELF_ASSIGN.category,
                severity,
                "self-assignment",
                ctx.module.file_id,
                expression_span,
            )
            .with_label("this assignment has no effect");

            // add fix for direct assignment only when source extraction is valid
            if *operator == ast::AssignOperator::Assign && !left_span.is_empty() {
                let left_text = ctx.get_span_text(left_span);
                if !left_text.is_empty() {
                    let replacement = left_text.to_string();
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, replacement)
                        .into_edits();
                    let fix = LintFix::suggestion("Remove self-assignment").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one assignment target is property-like.
fn assign_pattern_is_property_assignment_target(
    tree: &ast::NodeTree,
    assign_pattern_id: ast::LocalNodeId<ast::AssignPattern>,
) -> bool {
    let Some(expression_id) = assign_pattern_expression(tree, assign_pattern_id) else {
        return false;
    };

    let expression = tree.get(expression_id);

    matches!(
        expression,
        ast::Expression::QualifiedReference { path, .. } if path.segments.len() > 1
    ) || matches!(
        expression,
        ast::Expression::Member { .. }
            | ast::Expression::PrivateMember { .. }
            | ast::Expression::Index { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_simple_self_assign() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_detects_simple_self_assign.ds",
            r#"
x = x
"#,
        );
        test.result(result).assert_lint("no-self-assign");
    }

    #[test]
    fn test_detects_member_self_assign() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_detects_member_self_assign.ds",
            r#"
obj.x = obj.x
"#,
        );
        test.result(result).assert_lint("no-self-assign");
    }

    #[test]
    fn test_detects_index_self_assign() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_detects_index_self_assign.ds",
            r#"
arr[0] = arr[0]
"#,
        );
        test.result(result).assert_lint("no-self-assign");
    }

    #[test]
    fn test_allows_different_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_allows_different_assignment.ds",
            r#"
x = y
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }

    #[test]
    fn test_allows_different_member_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_allows_different_member_assignment.ds",
            r#"
obj.x = obj.y
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }

    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_allows_compound_assignment.ds",
            r#"
x += x
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }

    #[test]
    fn test_detects_logical_and_self_assign() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_detects_logical_and_self_assign.ds",
            r#"
x &&= x
"#,
        );
        test.result(result)
            .assert_lint("no-self-assign")
            .assert_has_no_fix("no-self-assign");
    }

    #[test]
    fn test_fix_self_assign() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign);
        let result = test.lint_ast(
            "no_self_assign/test_fix_self_assign.ds",
            r#"
x = x
"#,
        );
        test.result(result)
            .assert_lint("no-self-assign")
            .assert_suggested_fixed(
                r#"
x;
"#,
            );
    }

    #[test]
    fn test_allows_property_self_assign_when_props_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoSelfAssign).with_options(|options| {
            options.correctness.no_self_assign_check_properties = false;
        });
        let result = test.lint_ast(
            "no_self_assign/test_allows_property_self_assign_when_props_disabled.ds",
            r#"
obj.x = obj.x
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }
}
