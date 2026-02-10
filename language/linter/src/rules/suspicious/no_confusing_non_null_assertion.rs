use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow non-null assertion (`!`) next to optional chain (`?.`).
    ///
    /// Using `!` near `?` creates confusing code like `foo!?.bar` or `foo?.bar!`
    /// that is hard to read and likely indicates a logic error.
    #[lint(
        id = "no-confusing-non-null-assertion",
        code = "LU005",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoConfusingNonNullAssertion,
    "Disallow confusing non-null assertions"
}

impl LintRule for NoConfusingNonNullAssertion {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConfusingNonNullAssertion::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // check for `!` (Must) followed by optional chain (Maybe with Indirect position)
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // pattern: foo!?.bar (Must followed by optional member/index/call)
            if let ast::Expression::Must { left, .. } = expr {
                // check if the result is used with optional chaining
                if is_optional_chain_target(ctx, node_id) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintDiagnostic::new(
                        NO_CONFUSING_NON_NULL_ASSERTION.id,
                        NO_CONFUSING_NON_NULL_ASSERTION.code,
                        NO_CONFUSING_NON_NULL_ASSERTION.category,
                        severity,
                        "confusing non-null assertion before optional chain",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("`!` before `?.` is confusing");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) =
                            parenthesize_non_null_before_optional_chain_fix(ctx, node_id)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                    continue;
                }

                // pattern: foo!.bar where left is optional chain (foo?.baz!)
                if is_optional_chain(ctx, *left) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintDiagnostic::new(
                        NO_CONFUSING_NON_NULL_ASSERTION.id,
                        NO_CONFUSING_NON_NULL_ASSERTION.code,
                        NO_CONFUSING_NON_NULL_ASSERTION.category,
                        severity,
                        "confusing non-null assertion after optional chain",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("`!` after `?.` chain is confusing");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) =
                            parenthesize_optional_chain_before_non_null_fix(ctx, node_id, *left)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Build one safe fix by parenthesizing the non-null expression before optional chaining.
fn parenthesize_non_null_before_optional_chain_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.tree.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);
    if expression_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("({expression_text})");
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add grouping around non-null assertion").with_edits(edits))
}

/// Build one safe fix by parenthesizing the optional chain before non-null assertion.
fn parenthesize_optional_chain_before_non_null_fix(
    ctx: &LintModuleAstContext<'_>,
    must_expression_id: ast::LocalNodeId<ast::Expression>,
    optional_chain_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let optional_span = ctx.tree.get_span(optional_chain_id);
    let optional_text = ctx.get_span_text(optional_span);
    if optional_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("({optional_text})!");
    let must_span = ctx.tree.get_span(must_expression_id);
    let edits = ctx
        .edit_builder()
        .replace(must_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add grouping around optional chain").with_edits(edits))
}

/// Check if a node is used with optional chaining (is the left of a `?.` operation).
fn is_optional_chain_target(
    ctx: &LintModuleAstContext<'_>,
    node_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let Some(parent_id) = ctx.parents.get(node_id) else {
        return false;
    };

    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    let parent_expr_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent = ctx.tree.get(parent_expr_id);

    matches!(
        parent,
        ast::Expression::Maybe {
            position: ast::PostfixPosition::Indirect,
            ..
        }
    )
}

/// Check if an expression is or contains optional chaining.
fn is_optional_chain(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Maybe { .. } => true,
        ast::Expression::Member { left, .. } => is_optional_chain(ctx, *left),
        ast::Expression::Index { left, .. } => is_optional_chain(ctx, *left),
        ast::Expression::Call { left, .. } => is_optional_chain(ctx, *left),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_non_null_before_optional_chain() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_before_optional_chain.ds",
            r#"
const x = foo!.?bar
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion")
            .assert_has_fix("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_detects_non_null_after_optional_chain() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_after_optional_chain.ds",
            r#"
const x = foo?.bar!
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_allows_separate_non_null_and_optional() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_allows_separate_non_null_and_optional.ds",
            r#"
const x = foo!.bar
const y = baz?.qux
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_allows_simple_non_null() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_allows_simple_non_null.ds",
            r#"
const x = getValue()!
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_fix_parenthesizes_non_null_before_optional_chain() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_fix_parenthesizes_non_null_before_optional_chain.ds",
            r#"
const x = foo!.?bar
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion")
            .assert_safe_fixed(
                r#"
const x = (foo!).?bar
"#,
            );
    }

    #[test]
    fn test_fix_parenthesizes_optional_chain_before_non_null() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_fix_parenthesizes_optional_chain_before_non_null.ds",
            r#"
const x = foo?.bar!
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion")
            .assert_safe_fixed(
                r#"
const x = (foo?.bar)!;
"#,
            );
    }

    #[test]
    fn test_mutation_detects_nested_optional_chain_non_null_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_mutation_detects_nested_optional_chain_non_null_pattern.ds",
            r#"
const x = data?.user?.name!
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion")
            .assert_safe_fixed(
                r#"
const x = (data?.user?.name)!;
"#,
            );
    }
}
