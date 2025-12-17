use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer template literals over string concatenation.
    ///
    /// Use `` `Hello ${name}` `` instead of `"Hello " + name`.
    /// Template literals are more readable for string interpolation.
    #[lint(
        id = "prefer-template",
        code = "LY029",
        category = Style,
        level = Ast
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
                ctx.report(
                    LintDiagnostic::new(
                        PREFER_TEMPLATE.id,
                        PREFER_TEMPLATE.code,
                        PREFER_TEMPLATE.category,
                        severity,
                        "prefer template literal for string concatenation",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use template literal: `` `...${x}...` ``"),
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
}
