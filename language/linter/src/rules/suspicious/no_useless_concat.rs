use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_unwrap_parenthesized_syntax;
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary concatenation of string literals.
    ///
    /// Concatenating two string literals like `"a" + "b"` is unnecessary since
    /// they could be written as a single literal `"ab"`.
    #[lint(
        id = "no-useless-concat",
        code = "LU037",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessConcat,
    "Disallow useless string concatenation"
}

impl LintRule for NoUselessConcat {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessConcat::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // only check addition
            if *operator != ast::BinaryOperator::Add {
                continue;
            }

            // keep source parity: compare the innermost adjacent operands in concat chains
            let left_operand_id = concat_chain_right_operand(ctx, *left);
            let right_operand_id = concat_chain_left_operand(ctx, *right);
            if !is_string_literal(ctx, left_operand_id) || !is_string_literal(ctx, right_operand_id)
            {
                continue;
            }
            if !expressions_share_line(ctx, left_operand_id, right_operand_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_USELESS_CONCAT.id,
                NO_USELESS_CONCAT.code,
                NO_USELESS_CONCAT.category,
                severity,
                "useless string concatenation",
                ctx.module.file_id,
                expression_span,
            )
            .with_label("combine these into a single string literal");

            // keep fixes for direct literal pairs only:
            // nested chain rewrites need operator-local edits and are left diagnostic-only
            if ctx.compute_fixes
                && is_string_literal(ctx, *left)
                && is_string_literal(ctx, *right)
                && let Some(fix) = no_useless_concat_fix(ctx, node_id, *left, *right)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return the leftmost operand of a concat chain.
fn concat_chain_left_operand(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);
    if let ast::Expression::Binary {
        left,
        operator: ast::BinaryOperator::Add,
        ..
    } = expression
    {
        return concat_chain_left_operand(ctx, *left);
    }

    expression_id
}

/// Return the rightmost operand of a concat chain.
fn concat_chain_right_operand(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);
    if let ast::Expression::Binary {
        right,
        operator: ast::BinaryOperator::Add,
        ..
    } = expression
    {
        return concat_chain_right_operand(ctx, *right);
    }

    expression_id
}

/// Check if an expression is a string literal.
fn is_string_literal(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_)) => true,
        ast::Expression::Parenthesized { expression } => is_string_literal(ctx, *expression),
        _ => false,
    }
}

/// Return true when two expressions are on the same source line.
fn expressions_share_line(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left_span = ctx.tree.get_span(left_id);
    let right_span = ctx.tree.get_span(right_id);
    if left_span.file != right_span.file || left_span.end > right_span.start {
        return false;
    }

    let between_text =
        ctx.get_span_text(Span::new(left_span.file, left_span.end, right_span.start));
    !between_text.contains('\n')
}

/// Get the content of a string literal (without quotes).
/// Uses raw span text to preserve escape sequences.
fn get_string_content(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> String {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_)) => {
            let span = ctx.tree.get_span(expr_id);
            let text = ctx.get_span_text(span);
            // strip leading and trailing quotes
            if text.len() >= 2
                && ((text.starts_with('"') && text.ends_with('"'))
                    || (text.starts_with('\'') && text.ends_with('\'')))
            {
                text[1..text.len() - 1].to_string()
            } else {
                text.to_string()
            }
        }
        ast::Expression::Parenthesized { expression } => get_string_content(ctx, *expression),
        _ => String::new(),
    }
}

/// Build one safe fix for direct literal-literal concatenation.
fn no_useless_concat_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let left_content = get_string_content(ctx, left_id);
    let right_content = get_string_content(ctx, right_id);
    let combined = format!("\"{left_content}{right_content}\"");
    if combined.trim().is_empty() {
        return None;
    }

    let expression_span = ctx.tree.get_span(expression_id);
    let edits = ctx
        .edit_builder()
        .replace(expression_span, combined)
        .into_edits();
    Some(LintFix::safe("Combine string literals").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_concat() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_detects_string_concat.ds",
            r#"
const x = "hello" + "world";
"#,
        );
        test.result(result).assert_lint("no-useless-concat");
    }

    #[test]
    fn test_detects_string_concat_empty() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_detects_string_concat_empty.ds",
            r#"
const x = "" + "hello";
"#,
        );
        test.result(result).assert_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_variable_concat() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_allows_variable_concat.ds",
            r#"
const a = "hello";
const x = a + "world";
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_multiline_literal_concat() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_allows_multiline_literal_concat.ds",
            r#"
const x = "hello" +
"world";
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_number_addition() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_allows_number_addition.ds",
            r#"
const x = 1 + 2;
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_mixed_concat() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_allows_mixed_concat.ds",
            r#"
const x = "hello" + 42;
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_fix_string_concat() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_fix_string_concat.ds",
            r#"
const x = "hello" + "world";
"#,
        );
        test.result(result)
            .assert_lint("no-useless-concat")
            .assert_safe_fixed(
                r#"
const x = "helloworld";
"#,
            );
    }

    #[test]
    fn test_fix_empty_string() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_fix_empty_string.ds",
            r#"
const x = "" + "hello";
"#,
        );
        test.result(result)
            .assert_lint("no-useless-concat")
            .assert_safe_fixed(
                r#"
const x = "hello";
"#,
            );
    }

    #[test]
    fn test_detects_nested_literal_concat_chain() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint_ast(
            "no_useless_concat/test_detects_nested_literal_concat_chain.ds",
            r#"
const x = "a" + "b" + "c";
"#,
        );
        test.result(result)
            .assert_lint_count("no-useless-concat", 2);
    }
}
