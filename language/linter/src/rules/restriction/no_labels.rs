use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintMeta, LintRule, declare_lint};

declare_lint! {
    /// Disallow labeled statements.
    ///
    /// Labeled statements are rarely needed and can make control flow harder
    /// to understand. Consider restructuring with functions or different loop patterns.
    #[lint(
        id = "no-labels",
        code = "LR015",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoLabels,
    "Disallow labeled statements"
}

impl LintRule for NoLabels {
    fn meta(&self) -> &'static LintMeta {
        NoLabels::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let (message, label) = match expression {
                ast::Expression::Labelled { .. } => {
                    ("labeled statement is not allowed", "avoid using labels")
                }
                ast::Expression::Break { label: Some(_), .. } => (
                    "label in break statement is not allowed",
                    "remove the label from this break statement",
                ),
                ast::Expression::Continue { label: Some(_) } => (
                    "label in continue statement is not allowed",
                    "remove the label from this continue statement",
                ),
                _ => continue,
            };

            // resolve effective lint severity
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
                    message,
                    ctx.module.file_id,
                    span,
                )
                .with_label(label),
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
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_detects_labeled_statement.ts",
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
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_allows_unlabeled_loop.ts",
            r#"
for (let i = 0; i < 10; i++) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }

    #[test]
    fn test_detects_break_with_label() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_detects_break_with_label.ts",
            r#"
outer: for (let i = 0; i < 2; i++) {
    break outer;
}
"#,
        );
        test.result(result).assert_lint_count("no-labels", 2);
    }

    #[test]
    fn test_detects_continue_with_label() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_detects_continue_with_label.ts",
            r#"
outer: for (let i = 0; i < 2; i++) {
    continue outer;
}
"#,
        );
        test.result(result).assert_lint_count("no-labels", 2);
    }

    #[test]
    fn test_allows_break_without_label() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_allows_break_without_label.ts",
            r#"
for (let i = 0; i < 2; i++) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }
}
