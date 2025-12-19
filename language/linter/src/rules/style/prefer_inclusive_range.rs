use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        code = "LY055",
        category = Style,
        level = Ast,
        fixable = No,
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
                start: _,
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
            let end_expression = ctx.tree.get(*end);
            if is_add_one(ctx, end_expression) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

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
                    .with_label("replace `..x + 1` with `..=x`"),
                );
            }
        }
    }
}

/// Check if the expression is an addition of 1.
fn is_add_one(ctx: &LintModuleAstContext<'_>, expression: &Expression) -> bool {
    let Expression::Binary {
        operator,
        left: _,
        right,
    } = expression
    else {
        return false;
    };

    if *operator != BinaryOperator::Add {
        return false;
    }

    // check if right side is 1
    let right_expression = ctx.tree.get(*right);
    matches!(
        right_expression,
        Expression::ScalarLiteral(ScalarLiteral::Integer(1))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_exclusive_plus_one_detected() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..n + 1
"#,
        );
        test.result(result).assert_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_exclusive_plus_one_with_start_detected() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 1..len + 1
"#,
        );
        test.result(result).assert_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_inclusive_range_allowed() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..=n
"#,
        );
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_exclusive_without_plus_one_allowed() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..n
"#,
        );
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_exclusive_length_allowed() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..arr.length
"#,
        );
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_for_loop_plus_one_detected() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(n: int32) {
    for (const i of 0..n + 1) {
        print(i)
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_plus_two_allowed() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..n + 2
"#,
        );
        // adding 2 is a different pattern, allow it
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }

    #[test]
    fn test_minus_one_allowed() {
        let test = TestProgram::for_rule(PreferInclusiveRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..n - 1
"#,
        );
        // subtraction is handled by no-incomplete-range
        test.result(result).assert_no_lint("prefer-inclusive-range");
    }
}
