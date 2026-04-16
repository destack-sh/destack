use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_statement_ancestor, expression_unwrap_parenthesized_source_form,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

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
    fn meta(&self) -> &'static LintMeta {
        NoDelete::meta()
    }

    /// Check module AST nodes for delete expressions.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
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

/// Build an unsafe fix by rewriting one standalone delete statement.
fn no_delete_fix(
    ctx: &LintAstContext<'_>,
    delete_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    // require one standalone statement context around the delete expression
    expression_statement_ancestor(ctx.tree, ctx.parents, delete_id)?;

    // only rewrite assignable property targets
    let ast::Expression::Delete { value } = ctx.tree.get(delete_id) else {
        return None;
    };
    let target_id = expression_unwrap_parenthesized_source_form(ctx.tree, *value);
    let target_expression = ctx.tree.get(target_id);
    let target_is_assignable_property = matches!(
        target_expression,
        ast::Expression::Member { .. }
            | ast::Expression::PrivateMember { .. }
            | ast::Expression::Index { .. }
    );
    if !target_is_assignable_property {
        return None;
    }

    // replace the delete expression with an undefined assignment
    let delete_span = ctx.tree.get_span(delete_id);
    let target_span = ctx.tree.get_span(target_id);
    let target_text = ctx.get_span_text(target_span);
    let replacement = format!("{target_text} = undefined");
    let edits = ctx
        .edit_builder()
        .replace(delete_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Replace delete with undefined assignment").with_edits(edits))
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

    /// Unsafely rewrite standalone delete statements.
    #[test]
    fn test_fix_rewrites_delete_statement() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_fix_rewrites_delete_statement.ts",
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
item.value = undefined;
"#,
            );
    }

    /// Do not auto-fix delete expressions for bare bindings.
    #[test]
    fn test_no_fix_for_delete_identifier_statement() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_no_fix_for_delete_identifier_statement.ts",
            r#"
let item = { value: 1 };
delete item;
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_has_no_fix("no-delete");
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

    /// Mutation: rewrite indexed delete statements.
    #[test]
    fn test_mutation_fix_rewrites_index_delete_statement() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_mutation_fix_rewrites_index_delete_statement.ts",
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
items[1] = undefined;
"#,
            );
    }

    /// Unsafely rewrite parenthesized delete statements.
    #[test]
    fn test_fix_rewrites_parenthesized_delete_statement() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_fix_rewrites_parenthesized_delete_statement.ts",
            r#"
let item = { value: 1 };
(delete item.value);
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_unsafe_fixed(
                r#"
let item = { value: 1 };
(item.value = undefined);
"#,
            );
    }

    /// Unsafely rewrite delete statements with parenthesized targets.
    #[test]
    fn test_fix_rewrites_delete_with_parenthesized_target() {
        let test = TestProgram::for_rule_without_prelude(NoDelete);
        let result = test.lint_ast(
            "no_delete/test_fix_rewrites_delete_with_parenthesized_target.ts",
            r#"
let item = { value: 1 };
delete (item.value);
"#,
        );
        test.result(result)
            .assert_lint("no-delete")
            .assert_unsafe_fixed(
                r#"
let item = { value: 1 };
item.value = undefined;
"#,
            );
    }
}
