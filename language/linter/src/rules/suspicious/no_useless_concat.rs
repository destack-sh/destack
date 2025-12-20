use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary concatenation of string literals.
    ///
    /// Concatenating two string literals like `"a" + "b"` is unnecessary since
    /// they could be written as a single literal `"ab"`.
    #[lint(
        id = "no-useless-concat",
        code = "LU003",
        category = Suspicious,
        level = Ast,
        fixable = Always,
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

            // check if both sides are string literals
            if is_string_literal(ctx, *left) && is_string_literal(ctx, *right) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let expression_span = ctx.tree.get_span(node_id);

                // make fix: combine string literals
                let left_content = get_string_content(ctx, *left);
                let right_content = get_string_content(ctx, *right);
                let combined = format!("\"{left_content}{right_content}\"");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, combined)
                    .into_edits();
                let fix = LintFix::safe("Combine string literals").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_CONCAT.id,
                        NO_USELESS_CONCAT.code,
                        NO_USELESS_CONCAT.category,
                        severity,
                        "useless string concatenation",
                        ctx.module.file_id,
                        expression_span,
                    )
                    .with_label("combine these into a single string literal")
                    .with_fix(fix),
                );
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_concat() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" + "world";
"#,
        );
        test.result(result).assert_lint("no-useless-concat");
    }

    #[test]
    fn test_detects_string_concat_empty() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "" + "hello";
"#,
        );
        test.result(result).assert_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_variable_concat() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const a = "hello";
const x = a + "world";
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_number_addition() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1 + 2;
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_mixed_concat() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" + 42;
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_fix_string_concat() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
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
}
