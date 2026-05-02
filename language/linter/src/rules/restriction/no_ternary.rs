use crate::LintMeta;
use destack_ast::{self as ast, IfKind};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow ternary expressions.
    ///
    /// Ternary expressions can be harder to read than if-else statements,
    /// especially when nested. Use if-else for clarity.
    #[lint(
        id = "no-ternary",
        code = "LR028",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoTernary,
    "Disallow ternary expressions"
}

impl LintRule for NoTernary {
    fn meta(&self) -> &'static LintMeta {
        NoTernary::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::If { kind, .. } = expression else {
                continue;
            };
            if *kind != IfKind::Ternary {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_TERNARY.id,
                    NO_TERNARY.code,
                    NO_TERNARY.category,
                    severity,
                    "ternary expression is not allowed",
                    span,
                )
                .label("use if-else instead"),
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
        let test = TestProgram::for_rule_without_prelude(NoTernary);
        let result = test.lint_ast("no_ternary/test_detects_ternary.ts", "let x = a ? b : c;");
        test.result(result).assert_lint("no-ternary");
    }

    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule_without_prelude(NoTernary);
        let result = test.lint_ast(
            "no_ternary/test_allows_if_else.ts",
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
