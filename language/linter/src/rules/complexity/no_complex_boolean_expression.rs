use destack_ast::{self as ast, BinaryOperator, Expression, Path, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest simplifying complex boolean expressions.
    ///
    /// Detects boolean expressions that can be simplified:
    /// - Double negation: `!!x` can be simplified to `x` or `Boolean(x)`
    /// - Redundant terms: `a && a` or `a || a`
    /// - Contradictions: `a && !a` (always false) or `a || !a` (always true)
    #[lint(
        id = "no-complex-boolean-expression",
        code = "LX010",
        category = Complexity,
        level = Ast
    )]
    pub NoComplexBooleanExpression,
    "Suggest simplifying boolean expressions"
}

impl LintRule for NoComplexBooleanExpression {
    fn meta(&self) -> &'static crate::LintMeta {
        NoComplexBooleanExpression::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check for double negation: !!x
            if let Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } = expression
            {
                let inner = ctx.tree.get(*right);
                if let Expression::Unary {
                    operator: UnaryOperator::Not,
                    ..
                } = inner
                {
                    ctx.report(
                        LintDiagnostic::new(
                            NO_COMPLEX_BOOLEAN_EXPRESSION.id,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.code,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.category,
                            severity,
                            "double negation can be simplified",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("simplify to just the inner expression"),
                    );
                    continue;
                }
            }

            // check for redundant or contradictory binary expressions
            if let Expression::Binary {
                operator,
                left,
                right,
            } = expression
            {
                // only check logical and/or
                if !matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                    continue;
                }

                // check if both sides are the same expression (redundant)
                if expressions_equal(ctx, *left, *right) {
                    ctx.report(
                        LintDiagnostic::new(
                            NO_COMPLEX_BOOLEAN_EXPRESSION.id,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.code,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.category,
                            severity,
                            format!(
                                "redundant `{}` expression with identical operands",
                                if *operator == BinaryOperator::And {
                                    "&&"
                                } else {
                                    "||"
                                }
                            ),
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("simplify to just one operand"),
                    );
                    continue;
                }

                // check for contradiction: a && !a or a || !a
                if is_negation_of(ctx, *left, *right) || is_negation_of(ctx, *right, *left) {
                    let (result, suggestion) = if *operator == BinaryOperator::And {
                        ("always false", "replace with `false`")
                    } else {
                        ("always true", "replace with `true`")
                    };
                    ctx.report(
                        LintDiagnostic::new(
                            NO_COMPLEX_BOOLEAN_EXPRESSION.id,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.code,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.category,
                            severity,
                            format!("expression is {result}"),
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label(suggestion),
                    );
                }
            }
        }
    }
}

/// Return whether two expressions are structurally equal.
fn expressions_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<Expression>,
    right_id: ast::LocalNodeId<Expression>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);
    match (left, right) {
        // compare identifiers by name
        (
            Expression::Path {
                path: left_path, ..
            },
            Expression::Path {
                path: right_path, ..
            },
        ) => paths_equal(ctx, left_path, right_path),
        // compare scalar literals
        (Expression::ScalarLiteral(left_lit), Expression::ScalarLiteral(right_lit)) => {
            left_lit == right_lit
        }
        // #Cleanup: compare more complex expressions in lints? (see no_duplicate_case, no_dupe_else_if, ...)
        _ => false,
    }
}

/// Return whether two paths are equal.
fn paths_equal(ctx: &LintModuleAstContext<'_>, left: &Path, right: &Path) -> bool {
    if left.segments.len() != right.segments.len() {
        return false;
    }
    // compare each segment by resolving the string IDs
    for (left_seg, right_seg) in left.segments.iter().zip(right.segments.iter()) {
        let left_str = ctx.strings.get(*left_seg);
        let right_str = ctx.strings.get(*right_seg);
        if left_str.as_ref() != right_str.as_ref() {
            return false;
        }
    }
    true
}

/// Return whether `right` is the negation of `left`.
fn is_negation_of(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<Expression>,
    right_id: ast::LocalNodeId<Expression>,
) -> bool {
    let right = ctx.tree.get(right_id);
    if let Expression::Unary {
        operator: UnaryOperator::Not,
        right: inner_id,
    } = right
    {
        expressions_equal(ctx, left_id, *inner_id)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_double_negation() {
        let test = TestProgram::for_rule(NoComplexBooleanExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = !!value
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_redundant_and() {
        let test = TestProgram::for_rule(NoComplexBooleanExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = a && a
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_redundant_or() {
        let test = TestProgram::for_rule(NoComplexBooleanExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = b || b
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_contradiction_and() {
        let test = TestProgram::for_rule(NoComplexBooleanExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = a && !a
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_contradiction_or() {
        let test = TestProgram::for_rule(NoComplexBooleanExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = a || !a
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_allows_valid_expressions() {
        let test = TestProgram::for_rule(NoComplexBooleanExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = a && b
let y = a || b
let z = !value
"#,
        );
        test.result(result)
            .assert_no_lint("no-complex-boolean-expression");
    }
}
