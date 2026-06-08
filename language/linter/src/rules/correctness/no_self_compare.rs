use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{
    expressions_have_equivalent_source_form, is_binary_comparison_operator,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow comparing a value to itself.
    ///
    /// Comparing a variable to itself is usually a mistake or dead code.
    /// `x == x` is always true (except for NaN), and `x != x` is always false.
    #[lint(
        id = "no-self-compare",
        code = "LC025",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSelfCompare,
    "Disallow comparing a value to itself"
}

impl LintRule for NoSelfCompare {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoSelfCompare::meta()
    }

    /// Check module DIR nodes for self comparisons.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect expression nodes for self comparisons
        for (node_id, expression) in ctx.dir.iter_nodes_of_type::<dir::Expression>() {
            // filter to comparison binary expressions
            let dir::Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };

            // keep only comparison operators
            if !is_binary_comparison_operator(*operator) {
                continue;
            }

            // require equivalent source form on both sides
            if !expressions_have_equivalent_source_form(ctx, *left, *right) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);

            // skip disabled diagnostics
            if !severity.is_enabled() {
                continue;
            }

            // report self comparison diagnostic
            let span = ctx.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_SELF_COMPARE.id,
                    NO_SELF_COMPARE.code,
                    NO_SELF_COMPARE.category,
                    severity,
                    "comparing a value to itself",
                    span,
                )
                .label("both sides of this comparison are identical"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_self_equal() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_detects_self_equal.ds",
            r#"
let x = 1;
x == x;
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_detects_self_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_detects_self_not_equal.ds",
            r#"
let x = 1;
x != x;
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_detects_self_less() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_detects_self_less.ds",
            r#"
let x = 1;
x < x;
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_no_self_compare_different_vars() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_no_self_compare_different_vars.ds",
            r#"
let x = 1;
let y = 2;
x == y;
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_no_self_compare_arithmetic() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_no_self_compare_arithmetic.ds",
            r#"
let x = 1;
x + x;
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_detects_parenthesized_self_compare() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_detects_parenthesized_self_compare.ds",
            r#"
let x = 1;
(x) == x;
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_detects_member_self_compare_with_spacing() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_detects_member_self_compare_with_spacing.ds",
            r#"
let container = { value: 1 };
container.value >= container .value;
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_detects_index_self_compare() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_detects_index_self_compare.ds",
            r#"
let values = [1, 2, 3];
let index = 1;
values[index] === values[index];
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_allows_different_member_compare() {
        let test = TestProgram::for_rule_without_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "no_self_compare/test_allows_different_member_compare.ds",
            r#"
let container = { left: 1, right: 2 };
container.left >= container.right;
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }
}
