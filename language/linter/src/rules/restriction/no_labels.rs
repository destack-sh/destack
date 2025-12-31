use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow labeled statements.
    ///
    /// Labeled statements are rarely needed and can make control flow harder
    /// to understand. Consider restructuring with functions or different loop patterns.
    #[lint(
        id = "no-labels",
        code = "LR016",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoLabels,
    "Disallow labeled statements"
}

impl LintRule for NoLabels {
    fn meta(&self) -> &'static crate::LintMeta {
        NoLabels::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::Labelled { .. }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_LABELS.id,
                    NO_LABELS.code,
                    NO_LABELS.category,
                    severity,
                    "labeled statement is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("avoid using labels"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_labeled_statement() {
        let test = TestProgram::for_rule_without_builtins(NoLabels);
        let result = test.lint_ast(
            "test.ts",
            r#"
outer: for (let i = 0; i < 10; i++) {
    break outer;
}
"#,
        );
        test.result(result).assert_lint("no-labels");
    }

    #[test]
    fn test_allows_unlabeled_loop() {
        let test = TestProgram::for_rule_without_builtins(NoLabels);
        let result = test.lint_ast(
            "test.ts",
            r#"
for (let i = 0; i < 10; i++) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }
}
