use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

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
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedTernary,
    "Disallow nested ternary expressions"
}

impl LintRule for NoNestedTernary {
    fn meta(&self) -> &'static LintMeta {
        NoNestedTernary::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            // look for ternary expressions
            let dir::Expression::If {
                form: dir::IfForm::Ternary,
                condition,
                then_expression,
                else_expression,
            } = ctx.dir.get(node_id)
            else {
                continue;
            };

            // check if any child is also a ternary
            let condition_id = match condition {
                dir::IfCondition::Expression { condition } => *condition,
                dir::IfCondition::Let { .. } => continue,
            };
            let has_nested_ternary = is_ternary(ctx, condition_id)
                || is_ternary(ctx, *then_expression)
                || else_expression.map(|e| is_ternary(ctx, e)).unwrap_or(false);
            if has_nested_ternary {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let diagnostic = LintReport::new(
                    NO_NESTED_TERNARY.id,
                    NO_NESTED_TERNARY.code,
                    NO_NESTED_TERNARY.category,
                    severity,
                    "nested ternary expression",
                    ctx.dir.get_span(node_id),
                )
                .label("consider using if-else instead");

                ctx.report(diagnostic);
            }
        }
    }
}

/// Check if an expression is a ternary (possibly wrapped in parentheses).
fn is_ternary(ctx: &LintModuleContext<'_>, expr_id: dir::LocalNodeId<dir::Expression>) -> bool {
    let expr = ctx.dir.get(expr_id);
    match expr {
        dir::Expression::If {
            form: dir::IfForm::Ternary,
            ..
        } => true,
        dir::Expression::Parenthesized { expression } => is_ternary(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_ternary_in_then() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint(
            "no_nested_ternary/test_detects_nested_ternary_in_then.ds",
            r#"
const x = a ? b ? 1 : 2 : 3;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-ternary")
            .assert_has_no_fix("no-nested-ternary");
    }

    #[test]
    fn test_detects_nested_ternary_in_else() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint(
            "no_nested_ternary/test_detects_nested_ternary_in_else.ds",
            r#"
const x = a ? 1 : b ? 2 : 3;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-ternary")
            .assert_has_no_fix("no-nested-ternary");
    }

    #[test]
    fn test_detects_nested_ternary_in_condition() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint(
            "no_nested_ternary/test_detects_nested_ternary_in_condition.ds",
            r#"
const x = (a ? true : false) ? 1 : 2;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-ternary")
            .assert_has_no_fix("no-nested-ternary");
    }

    #[test]
    fn test_allows_simple_ternary() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTernary);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_nested_ternary/test_allows_separate_ternaries.ds",
            r#"
const x = a ? 1 : 2;
const y = b ? 3 : 4;
"#,
        );
        test.result(result).assert_no_lint("no-nested-ternary");
    }
}
