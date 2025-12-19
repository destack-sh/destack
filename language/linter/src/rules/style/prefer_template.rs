use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer template literals over string concatenation.
    ///
    /// Use `` `Hello ${name}` `` instead of `"Hello " + name`.
    /// Template literals are more readable for string interpolation.
    #[lint(
        id = "prefer-template",
        code = "LY029",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferTemplate,
    "Prefer template literals for string concatenation"
}

impl LintRule for PreferTemplate {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferTemplate::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for binary + operator
            let Expression::Binary {
                left,
                operator: BinaryOperator::Add,
                right,
            } = expr
            else {
                continue;
            };

            let left_expr = ctx.tree.get(*left);
            let right_expr = ctx.tree.get(*right);

            // check if either operand is a string literal
            let left_is_string = is_string_expression(left_expr);
            let right_is_string = is_string_expression(right_expr);

            // flag if at least one side is a string and the other is not
            // (if both are strings, no-useless-concat should catch it)
            if (left_is_string || right_is_string) && !(left_is_string && right_is_string) {
                let expression_span = ctx.tree.get_span(node_id);

                // make the fix: convert to template literal
                let replacement = if left_is_string {
                    // "str" + expr -> `str${expr}`
                    let str_content = get_string_content(ctx, *left);
                    let right_span = ctx.tree.get_span(*right);
                    let right_text = ctx.get_span_text(right_span);
                    let escaped = escape_for_template(&str_content);
                    format!("`{escaped}${{{right_text}}}`")
                } else {
                    // expr + "str" -> `${expr}str`
                    let str_content = get_string_content(ctx, *right);
                    let left_span = ctx.tree.get_span(*left);
                    let left_text = ctx.get_span_text(left_span);
                    let escaped = escape_for_template(&str_content);
                    format!("`${{{left_text}}}{escaped}`")
                };
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Convert to template literal").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_TEMPLATE.id,
                        PREFER_TEMPLATE.code,
                        PREFER_TEMPLATE.category,
                        severity,
                        "prefer template literal for string concatenation",
                        ctx.module.file_id,
                        expression_span,
                    )
                    .with_label("use template literal: `` `...${x}...` ``")
                    .with_fix(fix),
                );
            }
        }
    }
}

/// Check if an expression is a string literal.
fn is_string_expression(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    )
}

/// Get the string content from a string literal expression.
fn get_string_content(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> String {
    let expr = ctx.tree.get(expression_id);
    match expr {
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            ctx.strings.get(*string_id).as_ref().to_string()
        }
        // template expressions: just get the raw text minus backticks
        Expression::TemplateExpression { .. } => {
            let span = ctx.tree.get_span(expression_id);
            let text = ctx.get_span_text(span);
            // remove leading and trailing backticks
            if text.starts_with('`') && text.ends_with('`') && text.len() >= 2 {
                text[1..text.len() - 1].to_string()
            } else {
                text.to_string()
            }
        }
        _ => String::new(),
    }
}

/// Escape special characters for use in a template literal.
fn escape_for_template(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_concat() {
        let test = TestProgram::for_rule(PreferTemplate);
        let result = test.lint_ast(
            "test.ds",
            r#"
const greeting = "Hello " + name
"#,
        );
        test.result(result).assert_lint("prefer-template");
    }

    #[test]
    fn test_detects_concat_with_string_on_right() {
        let test = TestProgram::for_rule(PreferTemplate);
        let result = test.lint_ast(
            "test.ds",
            r#"
const greeting = name + " says hi"
"#,
        );
        test.result(result).assert_lint("prefer-template");
    }

    #[test]
    fn test_allows_template_literal() {
        let test = TestProgram::for_rule(PreferTemplate);
        let result = test.lint_ast(
            "test.ds",
            r#"
const greeting = `Hello ${name}`
"#,
        );
        test.result(result).assert_no_lint("prefer-template");
    }

    #[test]
    fn test_allows_number_addition() {
        let test = TestProgram::for_rule(PreferTemplate);
        let result = test.lint_ast(
            "test.ds",
            r#"
const sum = a + b
"#,
        );
        test.result(result).assert_no_lint("prefer-template");
    }

    #[test]
    fn test_allows_two_strings() {
        let test = TestProgram::for_rule(PreferTemplate);
        // two string literals should be caught by no-useless-concat
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" + "world"
"#,
        );
        test.result(result).assert_no_lint("prefer-template");
    }

    #[test]
    fn test_fix_string_on_left() {
        let test = TestProgram::for_rule(PreferTemplate);
        let result = test.lint_ast(
            "test.ds",
            r#"
const greeting = "Hello " + name
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const greeting = `Hello ${name}`;
"#,
            );
    }

    #[test]
    fn test_fix_string_on_right() {
        let test = TestProgram::for_rule(PreferTemplate);
        let result = test.lint_ast(
            "test.ds",
            r#"
const greeting = name + " says hi"
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const greeting = `${name} says hi`;
"#,
            );
    }
}
