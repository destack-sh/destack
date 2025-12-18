use destack_ast::{self as ast, BinaryOperator, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow comparing boolean expressions to boolean literals.
    ///
    /// Comparisons to `true` or `false` are unnecessary and can be simplified:
    /// - `x == true` → `x`
    /// - `x == false` → `!x`
    /// - `x != true` → `!x`
    /// - `x != false` → `x`
    #[lint(
        id = "no-boolean-literal-compare",
        code = "LY046",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoBooleanLiteralCompare,
    "Disallow comparing to boolean literals"
}

impl LintRule for NoBooleanLiteralCompare {
    fn meta(&self) -> &'static crate::LintMeta {
        NoBooleanLiteralCompare::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::Binary {
                operator,
                left,
                right,
            } = expression
            else {
                continue;
            };

            // only check equality/inequality operators
            if !matches!(
                operator,
                BinaryOperator::Equal
                    | BinaryOperator::EqualStrict
                    | BinaryOperator::NotEqual
                    | BinaryOperator::NotEqualStrict
            ) {
                continue;
            }

            let left_expression = ctx.tree.get(*left);
            let right_expression = ctx.tree.get(*right);

            let (is_left_bool, left_value) = get_boolean_literal(left_expression);
            let (is_right_bool, right_value) = get_boolean_literal(right_expression);

            // check if comparing to a boolean literal
            let (bool_value, on_left) = if is_left_bool {
                (left_value.unwrap(), true)
            } else if is_right_bool {
                (right_value.unwrap(), false)
            } else {
                continue;
            };

            let (message, suggestion) = get_message_and_suggestion(*operator, bool_value, on_left);

            ctx.report(
                LintDiagnostic::new(
                    NO_BOOLEAN_LITERAL_COMPARE.id,
                    NO_BOOLEAN_LITERAL_COMPARE.code,
                    NO_BOOLEAN_LITERAL_COMPARE.category,
                    severity,
                    message,
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label(suggestion),
            );
        }
    }
}

/// Check if an expression is a boolean literal and return its value.
fn get_boolean_literal(expression: &ast::Expression) -> (bool, Option<bool>) {
    if let ast::Expression::ScalarLiteral(ScalarLiteral::Boolean(value)) = expression {
        (true, Some(*value))
    } else {
        (false, None)
    }
}

/// Get the appropriate message and suggestion based on the comparison.
fn get_message_and_suggestion(
    operator: BinaryOperator,
    bool_value: bool,
    _on_left: bool,
) -> (&'static str, &'static str) {
    match (operator, bool_value) {
        // x == true or true == x
        (BinaryOperator::Equal | BinaryOperator::EqualStrict, true) => (
            "comparison to `true` is unnecessary",
            "use the expression directly",
        ),
        // x == false or false == x
        (BinaryOperator::Equal | BinaryOperator::EqualStrict, false) => (
            "comparison to `false` is unnecessary",
            "use `!expression` instead",
        ),
        // x != true or true != x
        (BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict, true) => (
            "comparison to `true` is unnecessary",
            "use `!expression` instead",
        ),
        // x != false or false != x
        (BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict, false) => (
            "comparison to `false` is unnecessary",
            "use the expression directly",
        ),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_equal_true_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x == true;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_equal_false_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x == false;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_not_equal_true_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x != true;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_not_equal_false_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x != false;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_strict_equal_true_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x === true;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_strict_not_equal_false_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x !== false;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_literal_on_left_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = true == x;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_false_on_left_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = false != x;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_non_boolean_comparison_allowed() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x == 42;
"#,
        );
        test.result(result)
            .assert_no_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_boolean_variable_comparison_allowed() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = x == y;
"#,
        );
        test.result(result)
            .assert_no_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_in_if_condition_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: bool) {
    if (x == true) {
        doSomething()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }

    #[test]
    fn test_in_ternary_condition_detected() {
        let test = TestProgram::for_rule(NoBooleanLiteralCompare);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = (x == false) ? 1 : 2;
"#,
        );
        test.result(result)
            .assert_lint("no-boolean-literal-compare");
    }
}
