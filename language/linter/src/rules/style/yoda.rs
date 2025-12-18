use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow "Yoda" conditions.
    ///
    /// Yoda conditions are comparisons where the literal value comes first,
    /// like `"red" === color` instead of `color === "red"`. While syntactically
    /// valid, they can be confusing and are less natural to read.
    #[lint(
        id = "yoda",
        code = "LY035",
        category = Style,
        level = Ast
    )]
    pub Yoda,
    "Disallow Yoda conditions"
}

impl LintRule for Yoda {
    fn meta(&self) -> &'static crate::LintMeta {
        Yoda::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };

            // only check comparison operators
            if !is_comparison_operator(operator) {
                continue;
            }

            // check for yoda condition: literal on left, non-literal on right
            let left_expression = ctx.tree.get(*left);
            let right_expression = ctx.tree.get(*right);
            if is_literal(left_expression) && !is_literal(right_expression) {
                ctx.report(
                    LintDiagnostic::new(
                        YODA.id,
                        YODA.code,
                        YODA.category,
                        severity,
                        "unexpected literal on the left side of comparison",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("move the literal to the right side"),
                );
            }
        }
    }
}

/// Check if an operator is a comparison operator.
fn is_comparison_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
    )
}

/// Check if an expression is a literal value.
fn is_literal(expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_) => true,
        Expression::TypeLiteral(_) => true,
        Expression::Parenthesized { expression: _ } => {
            // we don't recurse into parenthesized expressions to avoid
            // false positives on complex expressions like (a + b)
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_yoda_equality() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if ("red" === color) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_strict_equality() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (5 === x) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_less_than() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (10 < x) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_null_check() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (null === value) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_allows_normal_comparison() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (color === "red") {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_variable_comparison() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (a === b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_literal_to_literal() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (5 === 5) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_non_comparison_operators() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 5 + a;
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }
}
