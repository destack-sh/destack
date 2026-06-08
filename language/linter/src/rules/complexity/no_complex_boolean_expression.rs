use crate::LintMeta;
use destack_dir::{self as dir, BinaryOperator, Expression, UnaryOperator};
use destack_repository::LintSeverity;

use crate::rules::common::{
    expression_is_equal, expression_is_type_annotation, expression_unwrap_parenthesized_source_form,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest simplifying complex boolean expressions.
    ///
    /// Detects boolean expressions that can be simplified:
    /// - Double negation: `!!x` can be simplified to `x` or `Boolean(x)`
    /// - Redundant terms: `a && a` or `a || a`
    /// - Contradictions: `a && !a` (always false) or `a || !a` (always true)
    #[lint(
        id = "no-complex-boolean-expression",
        code = "LX015",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoComplexBooleanExpression,
    "Suggest simplifying boolean expressions"
}

impl LintRule for NoComplexBooleanExpression {
    fn meta(&self) -> &'static LintMeta {
        NoComplexBooleanExpression::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            // skip type expression contexts
            if expression_is_type_annotation(ctx.dir.tree(), node_id) {
                continue;
            }

            // check for double negation: !!x
            if let Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } = expression
            {
                let inner_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right);
                let inner = ctx.dir.get(inner_id);
                if let Expression::Unary {
                    operator: UnaryOperator::Not,
                    ..
                } = inner
                {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    ctx.report(
                        LintReport::new(
                            NO_COMPLEX_BOOLEAN_EXPRESSION.id,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.code,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.category,
                            severity,
                            "double negation can be simplified",
                            ctx.dir.get_span(node_id),
                        )
                        .label("simplify to just the inner expression"),
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
                if expression_is_equal(ctx, *left, *right) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    ctx.report(
                        LintReport::new(
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
                            ctx.dir.get_span(node_id),
                        )
                        .label("simplify to just one operand"),
                    );
                    continue;
                }

                // check for contradiction: a && !a or a || !a
                if is_negation_of(ctx, *left, *right) || is_negation_of(ctx, *right, *left) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let (result, suggestion) = if *operator == BinaryOperator::And {
                        ("always false", "replace with `false`")
                    } else {
                        ("always true", "replace with `true`")
                    };
                    ctx.report(
                        LintReport::new(
                            NO_COMPLEX_BOOLEAN_EXPRESSION.id,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.code,
                            NO_COMPLEX_BOOLEAN_EXPRESSION.category,
                            severity,
                            format!("expression is {result}"),
                            ctx.dir.get_span(node_id),
                        )
                        .label(suggestion),
                    );
                }
            }
        }
    }
}

/// Return whether `right` is the negation of `left`.
fn is_negation_of(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<Expression>,
    right_id: dir::LocalNodeId<Expression>,
) -> bool {
    let right_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), right_id);
    let right = ctx.dir.get(right_id);
    if let Expression::Unary {
        operator: UnaryOperator::Not,
        right: inner_id,
    } = right
    {
        let left_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), left_id);
        let inner_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *inner_id);
        expression_is_equal(ctx, left_id, inner_id)
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
        let test = TestProgram::for_rule_without_prelude(NoComplexBooleanExpression);
        let result = test.lint(
            "no_complex_boolean_expression/test_detects_double_negation.ds",
            r#"
let x = !!value
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_redundant_and() {
        let test = TestProgram::for_rule_without_prelude(NoComplexBooleanExpression);
        let result = test.lint(
            "no_complex_boolean_expression/test_detects_redundant_and.ds",
            r#"
let x = a && a
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_redundant_or() {
        let test = TestProgram::for_rule_without_prelude(NoComplexBooleanExpression);
        let result = test.lint(
            "no_complex_boolean_expression/test_detects_redundant_or.ds",
            r#"
let x = b || b
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_contradiction_and() {
        let test = TestProgram::for_rule_without_prelude(NoComplexBooleanExpression);
        let result = test.lint(
            "no_complex_boolean_expression/test_detects_contradiction_and.ds",
            r#"
let x = a && !a
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_detects_contradiction_or() {
        let test = TestProgram::for_rule_without_prelude(NoComplexBooleanExpression);
        let result = test.lint(
            "no_complex_boolean_expression/test_detects_contradiction_or.ds",
            r#"
let x = a || !a
"#,
        );
        test.result(result)
            .assert_lint("no-complex-boolean-expression");
    }

    #[test]
    fn test_allows_valid_expressions() {
        let test = TestProgram::for_rule_without_prelude(NoComplexBooleanExpression);
        let result = test.lint(
            "no_complex_boolean_expression/test_allows_valid_expressions.ds",
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
