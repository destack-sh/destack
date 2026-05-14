use destack_dir::{self as dir, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_optional_chain_target;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow extra non-null assertions.
    ///
    /// Using multiple non-null assertions (`value!!`) is redundant and likely
    /// a typo. A single `!` is sufficient to assert non-null.
    #[lint(
        id = "no-extra-non-null-assertion",
        code = "LU017",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoExtraNonNullAssertion,
    "Disallow extra non-null assertions"
}

impl LintRule for NoExtraNonNullAssertion {
    fn meta(&self) -> &'static LintMeta {
        NoExtraNonNullAssertion::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect expressions for nested non null assertions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            // require an outer non null assertion
            let Expression::Must { left, .. } = expression else {
                continue;
            };

            // require nested assertion or optional chain target
            let inner = ctx.dir.get(*left);
            let has_nested_non_null = matches!(inner, Expression::Must { .. });
            let has_optional_chain_target =
                expression_is_optional_chain_target(ctx.dir.tree(), node_id);
            if !has_nested_non_null && !has_optional_chain_target {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the nested assertion diagnostic
            let outer_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_EXTRA_NON_NULL_ASSERTION.id,
                NO_EXTRA_NON_NULL_ASSERTION.code,
                NO_EXTRA_NON_NULL_ASSERTION.category,
                severity,
                "extra non-null assertion",
                outer_span,
            )
            .label("remove the extra `!`");

            // replace the outer expression with the inner assertion text
            if ctx.compute_fixes {
                let inner_span = ctx.dir.get_span(*left);
                let inner_text = ctx.get_span_text(inner_span);
                let edits = ctx
                    .edit_builder()
                    .replace(outer_span, inner_text)
                    .into_edits();
                let fix = LintFix::safe("Remove extra `!`").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_double_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_detects_double_assertion.ts",
            r#"
const x = value!!;
"#,
        );
        test.result(result)
            .assert_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_detects_triple_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_detects_triple_assertion.ts",
            r#"
const x = value!!!;
"#,
        );
        // should detect at least one extra
        test.result(result)
            .assert_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_allows_single_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_allows_single_assertion.ts",
            r#"
const x = value!;
"#,
        );
        test.result(result)
            .assert_no_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_allows_no_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_allows_no_assertion.ts",
            r#"
const x = value;
"#,
        );
        test.result(result)
            .assert_no_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_allows_assertion_on_different_values() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_allows_assertion_on_different_values.ts",
            r#"
const x = a!.b!;
"#,
        );
        test.result(result)
            .assert_no_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_fix_removes_extra_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_fix_removes_extra_assertion.ts",
            r#"
const x = value!!
"#,
        );
        test.result(result)
            .assert_lint("no-extra-non-null-assertion")
            .assert_safe_fixed(
                r#"
const x = value!;
"#,
            );
    }

    #[test]
    fn test_detects_non_null_before_optional_chain() {
        let test = TestProgram::for_rule_without_prelude(NoExtraNonNullAssertion);
        let result = test.lint(
            "no_extra_non_null_assertion/test_detects_non_null_before_optional_chain.ts",
            r#"
const x = value!.?name
"#,
        );
        test.result(result)
            .assert_lint("no-extra-non-null-assertion")
            .assert_safe_fixed(
                r#"
const x = value.?name
"#,
            );
    }
}
