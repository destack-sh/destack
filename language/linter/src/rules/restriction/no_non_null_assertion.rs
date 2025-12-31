use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow non-null assertions (`!`).
    ///
    /// The `!` postfix operator bypasses type checking and can lead to
    /// runtime errors. Use proper null checks or optional chaining instead.
    #[lint(
        id = "no-non-null-assertion",
        code = "LR020",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoNonNullAssertion,
    "Disallow non-null assertions"
}

impl LintRule for NoNonNullAssertion {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNonNullAssertion::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::Must { .. }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_NON_NULL_ASSERTION.id,
                    NO_NON_NULL_ASSERTION.code,
                    NO_NON_NULL_ASSERTION.category,
                    severity,
                    "non-null assertion is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use null checks instead of `!`"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_non_null_assertion() {
        let test = TestProgram::for_rule_without_builtins(NoNonNullAssertion);
        let result = test.lint_ast("test.ts", "let x = foo!;");
        test.result(result).assert_lint("no-non-null-assertion");
    }

    #[test]
    fn test_detects_chained_non_null_assertion() {
        let test = TestProgram::for_rule_without_builtins(NoNonNullAssertion);
        let result = test.lint_ast("test.ts", "let x = foo!.bar;");
        test.result(result).assert_lint("no-non-null-assertion");
    }

    #[test]
    fn test_allows_optional_chaining() {
        let test = TestProgram::for_rule_without_builtins(NoNonNullAssertion);
        let result = test.lint_ast("test.ts", "let x = foo?.bar;");
        test.result(result).assert_no_lint("no-non-null-assertion");
    }

    #[test]
    fn test_allows_regular_access() {
        let test = TestProgram::for_rule_without_builtins(NoNonNullAssertion);
        let result = test.lint_ast("test.ts", "let x = foo.bar;");
        test.result(result).assert_no_lint("no-non-null-assertion");
    }
}
