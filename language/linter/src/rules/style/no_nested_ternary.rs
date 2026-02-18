use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow nested ternary expressions.
    ///
    /// Nested ternary expressions like `a ? b ? c : d : e` are hard to read
    /// and understand. Consider using if-else statements or extracting logic
    /// into separate variables or functions.
    #[lint(
        id = "no-nested-ternary",
        code = "LY022",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedTernary,
    "Disallow nested ternary expressions"
}

impl LintRule for NoNestedTernary {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNestedTernary::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // look for ternary expressions
            let ast::Expression::If {
                kind: ast::IfKind::Ternary,
                condition,
                then_expression,
                else_expression,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // check if any child is also a ternary
            let condition_id = match condition {
                ast::IfCondition::Expression { condition } => *condition,
                ast::IfCondition::Let { .. } => continue,
            };
            let has_nested_ternary = is_ternary(ctx, condition_id)
                || is_ternary(ctx, *then_expression)
                || else_expression.map(|e| is_ternary(ctx, e)).unwrap_or(false);
            if has_nested_ternary {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    NO_NESTED_TERNARY.id,
                    NO_NESTED_TERNARY.code,
                    NO_NESTED_TERNARY.category,
                    severity,
                    "nested ternary expression",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("consider using if-else instead");
                if ctx.compute_fixes
                    && let Some(fix) = no_nested_ternary_fix(ctx, node_id, condition_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Check if an expression is a ternary (possibly wrapped in parentheses).
fn is_ternary(ctx: &LintModuleAstContext<'_>, expr_id: ast::LocalNodeId<ast::Expression>) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::If {
            kind: ast::IfKind::Ternary,
            ..
        } => true,
        ast::Expression::Parenthesized { expression } => is_ternary(ctx, *expression),
        _ => false,
    }
}

/// Build an unsafe ternary-to-if-expression rewrite.
fn no_nested_ternary_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    condition_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let expression = ctx.tree.get(expression_id);
    let ast::Expression::If {
        kind: ast::IfKind::Ternary,
        then_expression,
        else_expression,
        ..
    } = expression
    else {
        return None;
    };
    let else_expression = (*else_expression)?;

    let condition_text = ctx
        .get_span_text(ctx.tree.get_span(condition_id))
        .to_string();
    let then_text = ctx
        .get_span_text(ctx.tree.get_span(*then_expression))
        .to_string();
    let else_text = ctx
        .get_span_text(ctx.tree.get_span(else_expression))
        .to_string();
    if condition_text.is_empty() || then_text.is_empty() || else_text.is_empty() {
        return None;
    }

    let replacement =
        format!("if ({condition_text}) {{\n    {then_text}\n}} else {{\n    {else_text}\n}}");
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(expression_id), replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Rewrite nested ternary to if/else expression").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_ternary_in_then() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint_ast(
            "no_nested_ternary/test_detects_nested_ternary_in_then.ds",
            r#"
const x = a ? b ? 1 : 2 : 3;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-ternary")
            .assert_unsafe_fixed(
                r#"
const x = if (a) {
    b ? 1 : 2;
} else {
    3
};
"#,
            );
    }

    #[test]
    fn test_detects_nested_ternary_in_else() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint_ast(
            "no_nested_ternary/test_detects_nested_ternary_in_else.ds",
            r#"
const x = a ? 1 : b ? 2 : 3;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-ternary")
            .assert_unsafe_fixed(
                r#"
const x = if (a) {
    1
} else {
    b ? 2 : 3;
};
"#,
            );
    }

    #[test]
    fn test_detects_nested_ternary_in_condition() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint_ast(
            "no_nested_ternary/test_detects_nested_ternary_in_condition.ds",
            r#"
const x = (a ? true : false) ? 1 : 2;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-ternary")
            .assert_unsafe_fixed(
                r#"
const x = if ((a ? true : false)) {
    1
} else {
    2
};
"#,
            );
    }

    #[test]
    fn test_allows_simple_ternary() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint_ast(
            "no_nested_ternary/test_allows_simple_ternary.ds",
            r#"
const x = condition ? 1 : 2;
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }

    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint_ast(
            "no_nested_ternary/test_allows_if_else.ds",
            r#"
const x = if (a) {
    if (b) { 1 } else { 2 }
} else {
    3
};
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }

    #[test]
    fn test_allows_separate_ternaries() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint_ast(
            "no_nested_ternary/test_allows_separate_ternaries.ds",
            r#"
const x = a ? 1 : 2;
const y = b ? 3 : 4;
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }
}
