use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow the use of `null`.
    ///
    /// Using only `undefined` for absent values leads to more consistent code.
    /// Consider using `undefined` instead of `null`.
    #[lint(
        id = "no-null",
        code = "LR019",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Off,
        stability = Stable
    )]
    pub NoNull,
    "Disallow null"
}

impl LintRule for NoNull {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNull::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if matches!(
                expression,
                ast::Expression::TypeLiteral(ast::TypeLiteral::Null)
            ) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.tree.get_span(node_id);
                let mut diagnostic = LintDiagnostic::new(
                    NO_NULL.id,
                    NO_NULL.code,
                    NO_NULL.category,
                    severity,
                    "use of `null`",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use `undefined` instead");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes {
                    let edits = ctx.edit_builder().replace(span, "undefined").into_edits();
                    let fix =
                        LintFix::r#unsafe("Replace `null` with `undefined`").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_null_literal() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast("no_null/test_detects_null_literal.ts", "const x = null;");
        test.result(result)
            .assert_lint("no-null")
            .assert_has_fix("no-null");
    }

    #[test]
    fn test_detects_null_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_detects_null_comparison.ts",
            "if (x === null) {}",
        );
        test.result(result).assert_lint("no-null");
    }

    #[test]
    fn test_allows_undefined() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast("no_null/test_allows_undefined.ts", "const x = undefined;");
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_optional() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_optional.ts",
            "const x: string | undefined = undefined;",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_fix_rewrites_null_literal() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_fix_rewrites_null_literal.ts",
            r#"
const value = null;
"#,
        );
        test.result(result)
            .assert_lint("no-null")
            .assert_unsafe_fixed(
                r#"
const value = undefined;
"#,
            );
    }

    #[test]
    fn test_mutation_fix_rewrites_null_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_mutation_fix_rewrites_null_comparison.ts",
            r#"
if (value === null) {}
"#,
        );
        test.result(result)
            .assert_lint("no-null")
            .assert_unsafe_fixed(
                r#"
if (value === undefined) {
}
"#,
            );
    }
}
