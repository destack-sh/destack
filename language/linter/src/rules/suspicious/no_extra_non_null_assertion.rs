use destack_ast::{self as ast, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow extra non-null assertions.
    ///
    /// Using multiple non-null assertions (`value!!`) is redundant and likely
    /// a typo. A single `!` is sufficient to assert non-null.
    #[lint(
        id = "no-extra-non-null-assertion",
        code = "LU022",
        category = Suspicious,
        level = Ast
    )]
    pub NoExtraNonNullAssertion,
    "Disallow extra non-null assertions"
}

impl LintRule for NoExtraNonNullAssertion {
    fn meta(&self) -> &'static crate::LintMeta {
        NoExtraNonNullAssertion::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check for Must expression (non-null assertion)
            let Expression::Must { left, .. } = expression else {
                continue;
            };

            // check if the inner expression is also a Must
            let inner = ctx.tree.get(*left);
            if !matches!(inner, Expression::Must { .. }) {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_EXTRA_NON_NULL_ASSERTION.id,
                    NO_EXTRA_NON_NULL_ASSERTION.code,
                    NO_EXTRA_NON_NULL_ASSERTION.category,
                    severity,
                    "extra non-null assertion",
                    ctx.module.file_id,
                    span,
                )
                .with_label("remove the extra `!`"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_double_assertion() {
        let test = TestProgram::for_rule(NoExtraNonNullAssertion);
        let result = test.lint_ast(
            "test.ts",
            r#"
const x = value!!;
"#,
        );
        test.result(result)
            .assert_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_detects_triple_assertion() {
        let test = TestProgram::for_rule(NoExtraNonNullAssertion);
        let result = test.lint_ast(
            "test.ts",
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
        let test = TestProgram::for_rule(NoExtraNonNullAssertion);
        let result = test.lint_ast(
            "test.ts",
            r#"
const x = value!;
"#,
        );
        test.result(result)
            .assert_no_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_allows_no_assertion() {
        let test = TestProgram::for_rule(NoExtraNonNullAssertion);
        let result = test.lint_ast(
            "test.ts",
            r#"
const x = value;
"#,
        );
        test.result(result)
            .assert_no_lint("no-extra-non-null-assertion");
    }

    #[test]
    fn test_allows_assertion_on_different_values() {
        let test = TestProgram::for_rule(NoExtraNonNullAssertion);
        let result = test.lint_ast(
            "test.ts",
            r#"
const x = a!.b!;
"#,
        );
        test.result(result)
            .assert_no_lint("no-extra-non-null-assertion");
    }
}
