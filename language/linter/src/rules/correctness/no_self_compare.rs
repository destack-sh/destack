use destack_dir::{BinaryOperator, Expression};
use destack_source::{FileId, ModuleId};
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
        level = Dir
    )]
    pub NoSelfCompare,
    "Disallow comparing a value to itself"
}

fn is_comparison_operator(op: BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
    )
}

impl LintRule for NoSelfCompare {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSelfCompare::meta()
    }

    fn check_module_dir(
        &self,
        file_id: FileId,
        _module_id: ModuleId,
        severity: LintSeverity,
        ctx: &mut LintModuleDirContext,
    ) {
        ctx.for_each::<Expression, _>(|tree, expression, span| {
            if let Expression::Binary {
                left,
                operator,
                right,
            } = expression
            {
                if !is_comparison_operator(*operator) {
                    return None;
                }

                // get the target symbols for both sides
                let left_expr = tree.get(*left);
                let right_expr = tree.get(*right);

                let left_symbol = left_expr.target_symbol();
                let right_symbol = right_expr.target_symbol();

                // if both sides reference the same symbol, it's a self-compare
                if let (Some(left_sym), Some(right_sym)) = (left_symbol, right_symbol)
                    && left_sym == right_sym
                {
                    return Some(
                        LintDiagnostic::new(
                            NO_SELF_COMPARE.id,
                            NO_SELF_COMPARE.code,
                            NO_SELF_COMPARE.category,
                            severity,
                            "comparing a value to itself",
                            file_id,
                            span,
                        )
                        .with_label("both sides of this comparison are identical"),
                    );
                }
            }
            None
        });
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
