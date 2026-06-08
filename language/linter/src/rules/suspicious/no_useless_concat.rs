use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;
use destack_source::Span;

use crate::rules::common::{
    expression_unwrap_parenthesized_source_form, single_quoted_string_literal,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary concatenation of string literals.
    ///
    /// Concatenating two string literals like `"a" + "b"` is unnecessary since
    /// they could be written as a single literal `"ab"`.
    #[lint(
        id = "no-useless-concat",
        code = "LU037",
        category = Suspicious,
        level = Dir,
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
    fn meta(&self) -> &'static LintMeta {
        NoUselessConcat::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.dir.get(node_id)
            else {
                continue;
            };

            // only check addition
            if *operator != dir::BinaryOperator::Add {
                continue;
            }

            // compare the innermost adjacent operands in concat chains
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

            let expression_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_USELESS_CONCAT.id,
                NO_USELESS_CONCAT.code,
                NO_USELESS_CONCAT.category,
                severity,
                "useless string concatenation",
                expression_span,
            )
            .label("combine these into a single string literal");

            // keep fixes for direct literal pairs only:
            // nested chain rewrites need operator-local edits and are left diagnostic-only
            if ctx.compute_fixes
                && is_string_literal(ctx, *left)
                && is_string_literal(ctx, *right)
                && let Some(fix) = no_useless_concat_fix(ctx, node_id, *left, *right)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return the leftmost operand of a concat chain.
fn concat_chain_left_operand(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);
    if let dir::Expression::Binary {
        left,
        operator: dir::BinaryOperator::Add,
        ..
    } = expression
    {
        return concat_chain_left_operand(ctx, *left);
    }

    expression_id
}

/// Return the rightmost operand of a concat chain.
fn concat_chain_right_operand(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);
    if let dir::Expression::Binary {
        right,
        operator: dir::BinaryOperator::Add,
        ..
    } = expression
    {
        return concat_chain_right_operand(ctx, *right);
    }

    expression_id
}

/// Check if an expression is a string literal.
fn is_string_literal(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expr = ctx.dir.get(expr_id);
    match expr {
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_)) => true,
        dir::Expression::Parenthesized { expression } => is_string_literal(ctx, *expression),
        _ => false,
    }
}

/// Return true when two expressions are on the same source line.
fn expressions_share_line(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let left_span = ctx.dir.get_span(left_id);
    let right_span = ctx.dir.get_span(right_id);
    if left_span.file != right_span.file || left_span.end > right_span.start {
        return false;
    }

    let between_text =
        ctx.get_span_text(Span::new(left_span.file, left_span.end, right_span.start));
    !between_text.contains('\n')
}

/// Get the semantic content of one string literal.
fn string_literal_content(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<String> {
    let expr = ctx.dir.get(expr_id);
    match expr {
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(string_id)) => {
            Some(ctx.strings.get(*string_id).to_string())
        }
        dir::Expression::Parenthesized { expression } => string_literal_content(ctx, *expression),
        _ => None,
    }
}

/// Build one safe fix for direct literal-literal concatenation.
fn no_useless_concat_fix(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let left_content = string_literal_content(ctx, left_id)?;
    let right_content = string_literal_content(ctx, right_id)?;
    let combined = single_quoted_string_literal(&format!("{left_content}{right_content}"));
    if combined.is_empty() {
        return None;
    }

    let expression_span = ctx.dir.get_span(expression_id);
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
    fn test_fix_preserves_quotes_and_escapes() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint(
            "no_useless_concat/test_fix_preserves_quotes_and_escapes.ds",
            r#"
const x = "a'\\n" + '"b"';
"#,
        );
        test.result(result)
            .assert_lint("no-useless-concat")
            .assert_safe_fixed(
                r#"
const x = "a\'\\\\n\"b\"";
"#,
            );
    }

    #[test]
    fn test_detects_nested_literal_concat_chain() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConcat);
        let result = test.lint(
            "no_useless_concat/test_detects_nested_literal_concat_chain.ds",
            r#"
const x = "a" + "b" + "c";
"#,
        );
        test.result(result)
            .assert_lint_count("no-useless-concat", 2);
    }
}
