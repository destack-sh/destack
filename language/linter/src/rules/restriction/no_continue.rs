use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow continue statements.
    ///
    /// The `continue` statement can make code harder to follow. Consider
    /// restructuring the loop logic or using early returns in helper functions.
    #[lint(
        id = "no-continue",
        code = "LR007",
        category = Restriction,
        level = Ast
    )]
    pub NoContinue,
    "Disallow continue statements"
}

impl LintRule for NoContinue {
    fn meta(&self) -> &'static crate::LintMeta {
        NoContinue::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::Continue { .. }) {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_CONTINUE.id,
                    NO_CONTINUE.code,
                    NO_CONTINUE.category,
                    severity,
                    "`continue` is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("restructure loop to avoid `continue`"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_continue() {
        let test = TestProgram::for_rule(NoContinue);
        let result = test.lint_ast(
            "test.ts",
            r#"
for (let i = 0; i < 10; i++) {
    if (i === 5) continue;
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("no-continue");
    }

    #[test]
    fn test_detects_labeled_continue() {
        let test = TestProgram::for_rule(NoContinue);
        let result = test.lint_ast(
            "test.ts",
            r#"
outer: for (let i = 0; i < 10; i++) {
    for (let j = 0; j < 10; j++) {
        continue outer;
    }
}
"#,
        );
        test.result(result).assert_lint("no-continue");
    }

    #[test]
    fn test_allows_break() {
        let test = TestProgram::for_rule(NoContinue);
        let result = test.lint_ast(
            "test.ts",
            r#"
for (let i = 0; i < 10; i++) {
    if (i === 5) break;
}
"#,
        );
        test.result(result).assert_no_lint("no-continue");
    }
}
