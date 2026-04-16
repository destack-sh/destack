use crate::LintMeta;
use destack_ast::{self as ast, AssignOperator, BinaryOperator, UnaryOperator};
use destack_workspace::{BitwiseOperator, LintSeverity};

use crate::rules::common::expression_unwrap_parenthesized_source_form;
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Disallow bitwise operators.
    ///
    /// Bitwise operators are often the result of typos (e.g., `|` instead of `||`)
    /// and can be confusing. If bitwise operations are needed, this rule can be
    /// disabled for specific files or projects.
    #[lint(
        id = "no-bitwise",
        code = "LR004",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoBitwise,
    "Disallow bitwise operators"
}

/// Return the configured bitwise operator kind for one binary operator.
fn bitwise_binary_operator(operator: &BinaryOperator) -> Option<BitwiseOperator> {
    match operator {
        BinaryOperator::ShiftLeft => Some(BitwiseOperator::ShiftLeft),
        BinaryOperator::SaturatingShiftLeft => Some(BitwiseOperator::SaturatingShiftLeft),
        BinaryOperator::ShiftRight => Some(BitwiseOperator::ShiftRight),
        BinaryOperator::UnsignedShiftRight => Some(BitwiseOperator::UnsignedShiftRight),
        BinaryOperator::ElementwiseAnd => Some(BitwiseOperator::And),
        BinaryOperator::ElementwiseXor => Some(BitwiseOperator::Xor),
        BinaryOperator::ElementwiseOr => Some(BitwiseOperator::Or),
        _ => None,
    }
}

/// Return the configured bitwise operator kind for one assignment operator.
fn bitwise_assign_operator(operator: &AssignOperator) -> Option<BitwiseOperator> {
    match operator {
        AssignOperator::ShiftLeftAssign => Some(BitwiseOperator::ShiftLeftAssign),
        AssignOperator::SaturatingShiftLeftAssign => {
            Some(BitwiseOperator::SaturatingShiftLeftAssign)
        }
        AssignOperator::ShiftRightAssign => Some(BitwiseOperator::ShiftRightAssign),
        AssignOperator::UnsignedShiftRightAssign => Some(BitwiseOperator::UnsignedShiftRightAssign),
        AssignOperator::ElementwiseAndAssign => Some(BitwiseOperator::AndAssign),
        AssignOperator::ElementwiseXorAssign => Some(BitwiseOperator::XorAssign),
        AssignOperator::ElementwiseOrAssign => Some(BitwiseOperator::OrAssign),
        _ => None,
    }
}

/// Return whether this expression is the configured `x | 0` int32 hint form.
fn expression_is_bitwise_int32_hint(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    operator: BitwiseOperator,
) -> bool {
    if !ctx.options.restriction.allow_bitwise_int32_hint || operator != BitwiseOperator::Or {
        return false;
    }

    let expression_id = expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    let ast::Expression::Binary { right, .. } = ctx.tree.get(expression_id) else {
        return false;
    };

    expression_is_zero_literal(ctx, *right)
}

/// Return whether this expression is a numeric zero literal.
fn expression_is_zero_literal(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);

    match ctx.tree.get(expression_id) {
        ast::Expression::ScalarLiteral(literal) => match literal {
            ast::ScalarLiteral::Integer(0) => true,
            ast::ScalarLiteral::Float(value) => *value == 0.0,
            _ => false,
        },
        _ => false,
    }
}

impl LintRule for NoBitwise {
    fn meta(&self) -> &'static LintMeta {
        NoBitwise::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let operator = match expression {
                ast::Expression::Binary { operator, .. } => bitwise_binary_operator(operator),
                ast::Expression::Unary { operator, .. } => {
                    if matches!(operator, UnaryOperator::ElementwiseNot) {
                        Some(BitwiseOperator::Not)
                    } else {
                        None
                    }
                }
                ast::Expression::Assign { operator, .. } => bitwise_assign_operator(operator),
                _ => None,
            };

            let Some(operator) = operator else {
                continue;
            };
            if ctx
                .options
                .restriction
                .allowed_bitwise_operators
                .contains(&operator)
                || expression_is_bitwise_int32_hint(ctx, node_id, operator)
            {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_BITWISE.id,
                    NO_BITWISE.code,
                    NO_BITWISE.category,
                    severity,
                    format!("bitwise operator `{}` is not allowed", operator.as_str()),
                    ctx.module.file_id,
                    span,
                )
                .with_label("avoid bitwise operators"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_bitwise_and() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_bitwise_and.ts", "let x = a & b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_or() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_bitwise_or.ts", "let x = a | b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_xor() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_bitwise_xor.ts", "let x = a ^ b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_not() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_bitwise_not.ts", "let x = ~a;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_shift_left() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_shift_left.ts", "let x = a << b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_saturating_shift_left() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast(
            "no_bitwise/test_detects_saturating_shift_left.ds",
            "let x = a <<| b;",
        );
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_shift_right() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_shift_right.ts", "let x = a >> b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_assign() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_detects_bitwise_assign.ts", "x &= 1;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_saturating_shift_left_assign() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast(
            "no_bitwise/test_detects_saturating_shift_left_assign.ds",
            "x <<|= 1;",
        );
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_unsigned_shift_right() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast(
            "no_bitwise/test_detects_unsigned_shift_right.ts",
            "let x = a >>> b;",
        );
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_unsigned_shift_right_assign() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast(
            "no_bitwise/test_detects_unsigned_shift_right_assign.ts",
            "x >>>= 1;",
        );
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_allows_logical_and() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_allows_logical_and.ts", "let x = a && b;");
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_allows_logical_or() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_allows_logical_or.ts", "let x = a || b;");
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_allows_arithmetic() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise);
        let result = test.lint_ast("no_bitwise/test_allows_arithmetic.ts", "let x = a + b * c;");
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_allows_configured_bitwise_not() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise).with_options(|options| {
            options
                .restriction
                .allowed_bitwise_operators
                .push(BitwiseOperator::Not);
        });
        let result = test.lint_ast(
            "no_bitwise/test_allows_configured_bitwise_not.ts",
            "let x = ~a;",
        );
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_allows_configured_int32_hint() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise).with_options(|options| {
            options.restriction.allow_bitwise_int32_hint = true;
        });
        let result = test.lint_ast(
            "no_bitwise/test_allows_configured_int32_hint.ts",
            "let x = (value) | 0;",
        );
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_still_flags_non_hint_bitwise_or_with_zero_on_left() {
        let test = TestProgram::for_rule_without_prelude(NoBitwise).with_options(|options| {
            options.restriction.allow_bitwise_int32_hint = true;
        });
        let result = test.lint_ast(
            "no_bitwise/test_still_flags_non_hint_bitwise_or_with_zero_on_left.ts",
            "let x = 0 | value;",
        );
        test.result(result).assert_lint("no-bitwise");
    }
}
