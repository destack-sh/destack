use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow comparing a value to itself.
    ///
    /// Comparing a variable to itself is usually a mistake or dead code.
    /// `x == x` is always true (except for NaN), and `x != x` is always false.
    #[lint(
        id = "no-self-compare",
        code = "LC004",
        category = Suspicious,
        level = Dir,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSelfCompare,
    "Disallow comparing a value to itself"
}

fn is_comparison_operator(op: dir::BinaryOperator) -> bool {
    matches!(
        op,
        dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqualStrict
            | dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
    )
}

impl LintRule for NoSelfCompare {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSelfCompare::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        for (node_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            // filter to comparison binary expressions
            let dir::Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };
            if !is_comparison_operator(*operator) {
                continue;
            }

            // get the target symbols for both sides
            let left_expr = ctx.tree.get(*left);
            let right_expr = ctx.tree.get(*right);
            let left_symbol = left_expr.target_symbol();
            let right_symbol = right_expr.target_symbol();

            // if both sides reference the same symbol, it's a self-compare
            if let (Some(left_sym), Some(right_sym)) = (left_symbol, right_symbol)
                && left_sym == right_sym
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_SELF_COMPARE.id,
                        NO_SELF_COMPARE.code,
                        NO_SELF_COMPARE.category,
                        severity,
                        "comparing a value to itself",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("both sides of this comparison are identical"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_self_equal() {
        let test = TestProgram::for_rule(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
let x = 1;
x == x;
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_detects_self_not_equal() {
        let test = TestProgram::for_rule(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
let x = 1;
x != x;
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_detects_self_less() {
        let test = TestProgram::for_rule(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
let x = 1;
x < x;
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_no_self_compare_different_vars() {
        let test = TestProgram::for_rule(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
let x = 1;
let y = 2;
x == y;
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_no_self_compare_arithmetic() {
        let test = TestProgram::for_rule(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
let x = 1;
x + x;
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }
}
