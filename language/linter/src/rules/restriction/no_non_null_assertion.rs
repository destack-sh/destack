use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_trailing_bang_span;
use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow non-null assertions (`!`).
    ///
    /// The `!` postfix operator bypasses type checking and can lead to
    /// runtime errors. Use proper null checks or optional chaining instead.
    #[lint(
        id = "no-non-null-assertion",
        code = "LR018",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Off,
        stability = Stable
    )]
    pub NoNonNullAssertion,
    "Disallow non-null assertions"
}

impl LintRule for NoNonNullAssertion {
    fn meta(&self) -> &'static LintMeta {
        NoNonNullAssertion::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::Must { .. }) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_NON_NULL_ASSERTION.id,
                NO_NON_NULL_ASSERTION.code,
                NO_NON_NULL_ASSERTION.category,
                severity,
                "non-null assertion is not allowed",
                span,
            )
            .label("use null checks instead of `!`");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(bang_span) = expression_trailing_bang_span(ctx, node_id)
            {
                let edits = ctx.edit_builder().delete(bang_span).into_edits();
                let fix = LintFix::suggestion("Remove non-null assertion").with_edits(edits);
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
    fn test_detects_non_null_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_detects_non_null_assertion.ts",
            "let x = foo!;",
        );
        test.result(result)
            .assert_lint("no-non-null-assertion")
            .assert_has_fix("no-non-null-assertion");
    }

    #[test]
    fn test_detects_chained_non_null_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_detects_chained_non_null_assertion.ts",
            "let x = foo!.bar;",
        );
        test.result(result).assert_lint("no-non-null-assertion");
    }

    #[test]
    fn test_allows_optional_chaining() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_allows_optional_chaining.ts",
            "let x = foo?.bar;",
        );
        test.result(result).assert_no_lint("no-non-null-assertion");
    }

    #[test]
    fn test_allows_regular_access() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_allows_regular_access.ts",
            "let x = foo.bar;",
        );
        test.result(result).assert_no_lint("no-non-null-assertion");
    }

    #[test]
    fn test_fix_removes_non_null_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_fix_removes_non_null_assertion.ts",
            r#"
let x = foo!
"#,
        );
        test.result(result)
            .assert_lint("no-non-null-assertion")
            .assert_suggested_fixed(
                r#"
let x = foo;
"#,
            );
    }

    #[test]
    fn test_fix_removes_chained_non_null_assertions() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_fix_removes_chained_non_null_assertions.ts",
            r#"
let x = foo!!.bar!
"#,
        );
        test.result(result)
            .assert_lint_count("no-non-null-assertion", 3)
            .assert_suggested_fixed(
                r#"
let x = foo.bar;
"#,
            );
    }

    #[test]
    fn test_mutation_detects_non_null_assertion_in_call_chain() {
        let test = TestProgram::for_rule_without_prelude(NoNonNullAssertion);
        let result = test.lint_ast(
            "no_non_null_assertion/test_mutation_detects_non_null_assertion_in_call_chain.ts",
            r#"
let value = getContainer()!.value!.toString()
"#,
        );
        test.result(result)
            .assert_lint_count("no-non-null-assertion", 2)
            .assert_suggested_fixed(
                r#"
let value = getContainer().value.toString();
"#,
            );
    }
}
