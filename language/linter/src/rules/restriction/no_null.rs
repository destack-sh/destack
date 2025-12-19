use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow the use of `null`.
    ///
    /// Using only `undefined` for absent values leads to more consistent code.
    /// Consider using `undefined` instead of `null`.
    #[lint(
        id = "no-null",
        code = "LR018",
        category = Restriction,
        level = Ast,
        fixable = No,
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
                ctx.report(
                    LintDiagnostic::new(
                        NO_NULL.id,
                        NO_NULL.code,
                        NO_NULL.category,
                        severity,
                        "use of `null`",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use `undefined` instead"),
                );
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
        let test = TestProgram::for_rule(NoNull);
        let result = test.lint_ast("test.ts", "const x = null;");
        test.result(result).assert_lint("no-null");
    }

    #[test]
    fn test_detects_null_comparison() {
        let test = TestProgram::for_rule(NoNull);
        let result = test.lint_ast("test.ts", "if (x === null) {}");
        test.result(result).assert_lint("no-null");
    }

    #[test]
    fn test_allows_undefined() {
        let test = TestProgram::for_rule(NoNull);
        let result = test.lint_ast("test.ts", "const x = undefined;");
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_optional() {
        let test = TestProgram::for_rule(NoNull);
        let result = test.lint_ast("test.ts", "const x: string | undefined = undefined;");
        test.result(result).assert_no_lint("no-null");
    }
}
