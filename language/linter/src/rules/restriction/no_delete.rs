use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow the delete operator.
    ///
    /// Deleting properties is often a source of confusing runtime behavior
    /// and can hurt performance due to object shape changes.
    #[lint(
        id = "no-delete",
        code = "LR036",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoDelete,
    "Disallow delete operator usage"
}

impl LintRule for NoDelete {
    /// Return lint metadata.
    fn meta(&self) -> &'static crate::LintMeta {
        NoDelete::meta()
    }

    /// Check module AST nodes for delete expressions.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk expressions to find delete usage
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // read the expression node
            let expression = ctx.tree.get(node_id);

            // skip non delete expressions
            if !matches!(expression, ast::Expression::Delete { .. }) {
                continue;
            }

            // honor per node severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // report the diagnostic
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_DELETE.id,
                    NO_DELETE.code,
                    NO_DELETE.category,
                    severity,
                    "delete operator is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("avoid using delete"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Detect delete usage.
    #[test]
    fn test_detects_delete_expression() {
        let test = TestProgram::for_rule_without_builtins(NoDelete);
        let result = test.lint_ast(
            "test.ts",
            r#"
let item = { value: 1 };
delete item.value;
"#,
        );
        test.result(result).assert_lint("no-delete");
    }

    /// Allow code without delete usage.
    #[test]
    fn test_allows_without_delete() {
        let test = TestProgram::for_rule_without_builtins(NoDelete);
        let result = test.lint_ast(
            "test.ts",
            r#"
let item = { value: 1 };
item.value = 2;
"#,
        );
        test.result(result).assert_no_lint("no-delete");
    }
}
