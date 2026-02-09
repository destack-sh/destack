use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer inclusive range syntax where applicable.
    ///
    /// When an exclusive range ends with `+ 1`, it's clearer to use an
    /// inclusive range instead. For example, `0..n + 1` can be written as `0..=n`.
    ///
    /// ```
    /// // bad
    /// for i of 0..n + 1 { }
    /// for i of 1..len + 1 { }
    ///
    /// // good
    /// for i of 0..=n { }
    /// for i of 1..=len { }
    /// ```
    ///
    /// Note: This lint does not trigger for patterns like `0..arr.length` which
    /// are idiomatic for exclusive iteration.
    #[lint(
        id = "prefer-inclusive-range",
        code = "LY042",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferInclusiveRange,
    "Prefer inclusive range syntax"
}

impl LintRule for PreferInclusiveRange {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferInclusiveRange::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // only check exclusive ranges
            let expression = ctx.tree.get(node_id);
            let Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } = expression
            else {
                continue;
            };
            if *is_inclusive {
                continue;
            }

            // check for `+ 1` at the end
            let Some(end_without_one) = add_one_left_expression_id(ctx, *end) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build a safe fix to convert `..x + 1` into `..=x`
            let range_span = ctx.tree.get_span(node_id);
            let start_span = ctx.tree.get_span(*start);
            let end_without_one_span = ctx.tree.get_span(end_without_one);
            let start_text = ctx.get_span_text(start_span);
            let end_without_one_text = ctx.get_span_text(end_without_one_span);
            let replacement = format!("{start_text}..={end_without_one_text}");
            let edits = ctx
                .edit_builder()
                .replace(range_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Replace exclusive range `..x + 1` with inclusive `..=x`")
                .with_edits(edits);

            ctx.report(
                LintDiagnostic::new(
                    PREFER_INCLUSIVE_RANGE.id,
                    PREFER_INCLUSIVE_RANGE.code,
                    PREFER_INCLUSIVE_RANGE.category,
                    severity,
                    "use inclusive range `..=` instead of exclusive range with `+ 1`",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("replace `..x + 1` with `..=x`")
                .with_fix(fix),
            );
        }
    }
}

/// Return the left expression when the expression is `left + 1`.
fn add_one_left_expression_id(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let expression_id = unwrap_parenthesized_expression(ctx, expression_id);
    let expression = ctx.tree.get(expression_id);

    let Expression::Binary {
        operator,
        left,
        right,
    } = expression
    else {
        return None;
    };

    if *operator != BinaryOperator::Add {
        return None;
    }

    // check if right side is 1
    let right_id = unwrap_parenthesized_expression(ctx, *right);
    let right_expression = ctx.tree.get(right_id);
    if matches!(
        right_expression,
        Expression::ScalarLiteral(ScalarLiteral::Integer(1))
    ) {
        return Some(*left);
    }

    None
}

/// Unwrap one or more parenthesized expressions.
fn unwrap_parenthesized_expression(
    ctx: &LintModuleAstContext<'_>,
    mut expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    loop {
        let expression = ctx.tree.get(expression_id);
        let Expression::Parenthesized { expression } = expression else {
            return expression_id;
        };
        expression_id = *expression;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_exclusive_plus_one_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_exclusive_plus_one_detected.ds",
            r#"
const range = 0..n + 1
"#,
        );
        test.result(result)
            .assert_lint("prefer-inclusive-range")
            .assert_has_fix("prefer-inclusive-range")
            .assert_safe_fixed(
                r#"
const range = 0..=n;
"#,
            );
    }

    #[test]
    fn test_exclusive_plus_one_with_start_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_exclusive_plus_one_with_start_detected.ds",
            r#"
const range = 1..len + 1
"#,
        );
        test.result(result).assert_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_inclusive_range_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_inclusive_range_allowed.ds",
            r#"
const range = 0..=n
"#,
        );
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_exclusive_without_plus_one_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_exclusive_without_plus_one_allowed.ds",
            r#"
const range = 0..n
"#,
        );
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_exclusive_length_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_exclusive_length_allowed.ds",
            r#"
const range = 0..arr.length
"#,
        );
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_for_loop_plus_one_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_for_loop_plus_one_detected.ds",
            r#"
function foo(n: int32) {
    for (const i of 0..n + 1) {
        print(i)
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-inclusive-range")
            .assert_has_fix("prefer-inclusive-range")
            .assert_safe_fixed(
                r#"
function foo(n: int32) {
    for (const i of 0..=n) {
        print(i)
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_parenthesized_plus_one_endpoint() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_fix_parenthesized_plus_one_endpoint.ds",
            r#"
const range = 0..(n + 1)
"#,
        );
        test.result(result)
            .assert_lint("prefer-inclusive-range")
            .assert_has_fix("prefer-inclusive-range")
            .assert_safe_fixed(
                r#"
const range = 0..=n;
"#,
            );
    }

    #[test]
    fn test_plus_two_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_plus_two_allowed.ds",
            r#"
const range = 0..n + 2
"#,
        );
        // adding 2 is a different pattern, allow it
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_minus_one_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferInclusiveRange);
        let result = test.lint_ast(
            "prefer_inclusive_range/test_minus_one_allowed.ds",
            r#"
const range = 0..n - 1
"#,
        );
        // subtraction is handled by no-incomplete-range
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }
}
