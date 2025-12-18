use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow expressions where the operation doesn't affect the value.
    ///
    /// Binary expressions with certain operand combinations always produce the same
    /// result regardless of the input. This includes comparisons of a value to itself
    /// with certain operators, and operations that have no effect.
    #[lint(
        id = "no-constant-binary-expression",
        code = "LC006",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoConstantBinaryExpression,
    "Disallow expressions that always produce the same result"
}

impl LintRule for NoConstantBinaryExpression {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConstantBinaryExpression::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // check for constant results
            if let Some(message) = check_constant_result(ctx, *left, *operator, *right) {
                let span = ctx.tree.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_CONSTANT_BINARY_EXPRESSION.id,
                        NO_CONSTANT_BINARY_EXPRESSION.code,
                        NO_CONSTANT_BINARY_EXPRESSION.category,
                        severity,
                        message,
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("this expression always produces the same result"),
                );
            }
        }
    }
}

/// Check if a binary expression produces a constant result.
fn check_constant_result(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    operator: ast::BinaryOperator,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> Option<&'static str> {
    // check for `x || x` or `x && x` where both sides are identical literals
    if matches!(operator, ast::BinaryOperator::Or | ast::BinaryOperator::And)
        && are_identical_literals(ctx, left_id, right_id)
    {
        return Some("logical operation on identical operands");
    }

    // check for `new X() === new X()` (always false for object comparisons)
    if matches!(
        operator,
        ast::BinaryOperator::EqualStrict | ast::BinaryOperator::NotEqualStrict
    ) {
        let left = ctx.tree.get(left_id);
        let right = ctx.tree.get(right_id);
        if matches!(left, ast::Expression::New { .. })
            && matches!(right, ast::Expression::New { .. })
        {
            return Some("comparing two new objects always produces the same result");
        }
    }

    // check for `{} === {}` or `[] === []` (always false)
    if matches!(
        operator,
        ast::BinaryOperator::EqualStrict | ast::BinaryOperator::NotEqualStrict
    ) {
        let left = ctx.tree.get(left_id);
        let right = ctx.tree.get(right_id);
        let left_is_object = matches!(
            left,
            ast::Expression::ObjectExpression { .. } | ast::Expression::ArrayExpression { .. }
        );
        let right_is_object = matches!(
            right,
            ast::Expression::ObjectExpression { .. } | ast::Expression::ArrayExpression { .. }
        );
        if left_is_object && right_is_object {
            return Some("comparing two object literals always produces the same result");
        }
    }

    // check for string + undefined or string + null
    if operator == ast::BinaryOperator::Add {
        let left = ctx.tree.get(left_id);
        let right = ctx.tree.get(right_id);
        let left_is_string = matches!(
            left,
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
                | ast::Expression::TemplateExpression { .. }
        );
        let right_is_nullish = is_nullish(right);
        if left_is_string && right_is_nullish {
            return Some("string concatenation with null/undefined");
        }
        let right_is_string = matches!(
            right,
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
                | ast::Expression::TemplateExpression { .. }
        );
        let left_is_nullish = is_nullish(left);
        if right_is_string && left_is_nullish {
            return Some("string concatenation with null/undefined");
        }
    }

    None
}

/// Check if two expressions are identical literals.
fn are_identical_literals(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    match (left, right) {
        (ast::Expression::ScalarLiteral(l), ast::Expression::ScalarLiteral(r)) => l == r,
        (ast::Expression::TypeLiteral(l), ast::Expression::TypeLiteral(r)) => l == r,
        (
            ast::Expression::Parenthesized { expression: l },
            ast::Expression::Parenthesized { expression: r },
        ) => are_identical_literals(ctx, *l, *r),
        (ast::Expression::Parenthesized { expression: l }, _) => {
            are_identical_literals(ctx, *l, right_id)
        }
        (_, ast::Expression::Parenthesized { expression: r }) => {
            are_identical_literals(ctx, left_id, *r)
        }
        _ => false,
    }
}

/// Check if an expression is nullish (null or undefined).
fn is_nullish(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::TypeLiteral(ast::TypeLiteral::Null | ast::TypeLiteral::Undefined)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_logical_or_same_literal() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
true || true;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_logical_and_same_literal() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
false && false;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_new_object_comparison() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
new Foo() === new Foo();
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_object_literal_comparison() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = {} === {};
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_array_literal_comparison() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
[] === [];
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_string_plus_null() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
"hello" + null;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_string_plus_undefined() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
"hello" + undefined;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_different_literals_or() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
true || false;
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_variable_comparison() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = {};
let y = {};
x === y;
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_string_concatenation() {
        let test = TestProgram::for_rule(NoConstantBinaryExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
"hello" + "world";
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }
}
