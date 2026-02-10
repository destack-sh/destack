use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow the delete operator.
    ///
    /// Deleting properties is often a source of confusing runtime behavior
    /// and can hurt performance due to object shape changes.
    #[lint(
        id = "no-delete",
        code = "LR010",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
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
            let mut diagnostic = LintDiagnostic::new(
                NO_DELETE.id,
                NO_DELETE.code,
                NO_DELETE.category,
                severity,
                "delete operator is not allowed",
                ctx.module.file_id,
                span,
            )
            .with_label("avoid using delete");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(fix) = no_delete_fix(ctx, node_id)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build an unsafe fix by removing one standalone delete statement.
fn no_delete_fix(
    ctx: &LintModuleAstContext<'_>,
    delete_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let parent_id = ctx.parents.get(delete_id)?;
    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return None;
    }

    let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent_expression = ctx.tree.get(parent_expression_id);
    let ast::Expression::Statement(statement_id) = parent_expression else {
        return None;
    };
    if *statement_id != delete_id {
        return None;
    }

    let statement_span = ctx.tree.get_span(parent_expression_id);
    let edits = ctx.edit_builder().delete(statement_span).into_edits();
    Some(LintFix::r#unsafe("Remove delete statement").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Detect delete usage.
    #[test]
    fn test_detects_delete_expression() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_detects_delete_expression.ts",
            r#"
let item = { value: 1 };
delete item.value;
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_has_fix("no-delete");
    }

    /// Allow code without delete usage.
    #[test]
    fn test_allows_without_delete() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_allows_without_delete.ts",
            r#"
let item = { value: 1 };
item.value = 2;
"#,
        );
        test.result(result).assert_no_lint("no-delete");
    }

    /// Unsafely remove standalone delete statements.
    #[test]
    fn test_fix_removes_delete_statement() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_fix_removes_delete_statement.ts",
            r#"
let item = { value: 1 };
delete item.value;
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_unsafe_fixed(
                r#"
let item = { value: 1 };
"#,
            );
    }

    /// Do not auto-fix delete expressions when the value is used.
    #[test]
    fn test_no_fix_when_delete_result_is_used() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_no_fix_when_delete_result_is_used.ts",
            r#"
let item = { value: 1 };
let removed = delete item.value;
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_has_no_fix("no-delete");
    }

    /// Mutation: remove indexed delete statements.
    #[test]
    fn test_mutation_fix_removes_index_delete_statement() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_mutation_fix_removes_index_delete_statement.ts",
            r#"
let items = [1, 2, 3];
delete items[1];
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
"#,
            );
    }
}
