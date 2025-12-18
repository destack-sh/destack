use destack_ast::{self as ast, IfKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow ternary expressions.
    ///
    /// Ternary expressions can be harder to read than if-else statements,
    /// especially when nested. Use if-else for clarity.
    #[lint(
        id = "no-ternary",
        code = "LR009",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoTernary,
    "Disallow ternary expressions"
}

impl LintRule for NoTernary {
    fn meta(&self) -> &'static crate::LintMeta {
        NoTernary::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::If { kind, .. } = expression else {
                continue;
            };
            if *kind != IfKind::Ternary {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_TERNARY.id,
                    NO_TERNARY.code,
                    NO_TERNARY.category,
                    severity,
                    "ternary expression is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use if-else instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_ternary() {
        let test = TestProgram::for_rule(NoTernary);
        let result = test.lint_ast("test.ts", "let x = a ? b : c;");
        test.result(result).assert_lint("no-ternary");
    }

    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule(NoTernary);
        let result = test.lint_ast(
            "test.ts",
            r#"
let x;
if (a) {
    x = b;
} else {
    x = c;
}
"#,
        );
        test.result(result).assert_no_lint("no-ternary");
    }
}
