use crate::LintMeta;
use destack_dir::{self as dir, BinaryOperator, ScalarLiteral, UnaryOperator};
use destack_repository::LintSeverity;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer unary negation over multiplying by -1.
    ///
    /// Using `-x` is clearer and more concise than `x * -1` or `-1 * x`.
    /// This also applies to saturating and wrapping multiplication variants.
    #[lint(
        id = "prefer-unary-negation",
        code = "LY061",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferUnaryNegation,
    "Prefer unary negation over multiplying by -1"
}

impl LintRule for PreferUnaryNegation {
    fn meta(&self) -> &'static LintMeta {
        PreferUnaryNegation::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let dir::Expression::Binary {
                operator,
                left,
                right,
            } = expression
            else {
                continue;
            };

            // check for multiplication
            if !matches!(operator, BinaryOperator::Multiply) {
                continue;
            }

            let left_expression = ctx.dir.get(*left);
            let right_expression = ctx.dir.get(*right);

            // check if either side is -1 (can be literal -1 or unary negate of 1)
            let left_is_neg_one = is_negative_one(ctx, left_expression, *left);
            let right_is_neg_one = is_negative_one(ctx, right_expression, *right);

            if left_is_neg_one || right_is_neg_one {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // make fix: use unary negation
                let expression_span = ctx.dir.get_span(node_id);
                let other_id = if left_is_neg_one { *right } else { *left };
                let other_span = ctx.dir.get_span(other_id);
                let other_text = ctx.get_span_text(other_span);
                let replacement = format!("-{other_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Use unary negation").with_edits(edits);

                ctx.report(
                    LintReport::new(
                        PREFER_UNARY_NEGATION.id,
                        PREFER_UNARY_NEGATION.code,
                        PREFER_UNARY_NEGATION.category,
                        severity,
                        "prefer unary negation over multiplying by -1",
                        expression_span,
                    )
                    .label("use `-x` instead")
                    .fix(fix),
                );
            }
        }
    }
}

/// Check if an expression represents the value -1.
fn is_negative_one(
    ctx: &LintModuleContext<'_>,
    expression: &dir::Expression,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // check for literal -1 (some languages might parse it directly as a negative literal)
    if let dir::Expression::ScalarLiteral(ScalarLiteral::Integer(value)) = expression
        && *value == -1
    {
        return true;
    }

    // check for unary negate of 1
    if let dir::Expression::Unary { operator, right } = expression
        && *operator == UnaryOperator::Negate
    {
        let inner = ctx.dir.get(*right);
        if is_one(inner) {
            return true;
        }
    }

    // check for parenthesized -1
    if let dir::Expression::Parenthesized {
        expression: inner_expression_id,
    } = expression
    {
        let inner = ctx.dir.get(*inner_expression_id);
        return is_negative_one(ctx, inner, *inner_expression_id);
    }

    // check the original expression ID for more context
    let _ = expression_id; // we've already handled this through the expression parameter

    false
}

/// Check if an expression is the literal 1.
fn is_one(expression: &dir::Expression) -> bool {
    if let dir::Expression::ScalarLiteral(ScalarLiteral::Integer(value)) = expression {
        return *value == 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_multiply_by_negative_one_right_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_by_negative_one_right_detected.ds",
            r#"
const result = x * -1;
"#,
        );
        test.result(result).assert_lint("prefer-unary-negation");
    }

    #[test]
    fn test_multiply_by_negative_one_left_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_by_negative_one_left_detected.ds",
            r#"
const result = -1 * x;
"#,
        );
        test.result(result).assert_lint("prefer-unary-negation");
    }

    #[test]
    fn test_multiply_by_negative_one_parenthesized_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_by_negative_one_parenthesized_detected.ds",
            r#"
const result = x * (-1);
"#,
        );
        test.result(result).assert_lint("prefer-unary-negation");
    }

    #[test]
    fn test_multiply_by_other_number_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_by_other_number_allowed.ds",
            r#"
const result = x * 2;
"#,
        );
        test.result(result).assert_no_lint("prefer-unary-negation");
    }

    #[test]
    fn test_multiply_by_negative_two_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_by_negative_two_allowed.ds",
            r#"
const result = x * -2;
"#,
        );
        test.result(result).assert_no_lint("prefer-unary-negation");
    }

    #[test]
    fn test_multiply_two_variables_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_two_variables_allowed.ds",
            r#"
const result = x * y;
"#,
        );
        test.result(result).assert_no_lint("prefer-unary-negation");
    }

    #[test]
    fn test_unary_negation_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_unary_negation_allowed.ds",
            r#"
const result = -x;
"#,
        );
        test.result(result).assert_no_lint("prefer-unary-negation");
    }

    #[test]
    fn test_multiply_by_one_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_multiply_by_one_allowed.ds",
            r#"
const result = x * 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-unary-negation");
    }

    #[test]
    fn test_fix_multiply_by_negative_one_right() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_fix_multiply_by_negative_one_right.ds",
            r#"
const result = x * -1
"#,
        );
        test.result(result)
            .assert_lint("prefer-unary-negation")
            .assert_safe_fixed(
                r#"
const result = -x;
"#,
            );
    }

    #[test]
    fn test_fix_multiply_by_negative_one_left() {
        let test = TestProgram::for_rule_without_prelude(PreferUnaryNegation);
        let result = test.lint(
            "prefer_unary_negation/test_fix_multiply_by_negative_one_left.ds",
            r#"
const result = -1 * x
"#,
        );
        test.result(result)
            .assert_lint("prefer-unary-negation")
            .assert_safe_fixed(
                r#"
const result = -x;
"#,
            );
    }
}
