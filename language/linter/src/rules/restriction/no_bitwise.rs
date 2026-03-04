use destack_ast::{self as ast, AssignOperator, BinaryOperator, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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

fn bitwise_binary_text(operator: &BinaryOperator) -> Option<&'static str> {
    match operator {
        BinaryOperator::ShiftLeft => Some("<<"),
        BinaryOperator::SaturatingShiftLeft => Some("<<<"),
        BinaryOperator::ShiftRight => Some(">>"),
        BinaryOperator::UnsignedShiftRight => Some(">>>"),
        BinaryOperator::ElementwiseAnd => Some("&"),
        BinaryOperator::ElementwiseXor => Some("^"),
        BinaryOperator::ElementwiseOr => Some("|"),
        _ => None,
    }
}

fn bitwise_assign_text(operator: &AssignOperator) -> Option<&'static str> {
    match operator {
        AssignOperator::ShiftLeftAssign => Some("<<="),
        AssignOperator::SaturatingShiftLeftAssign => Some("<<<="),
        AssignOperator::ShiftRightAssign => Some(">>="),
        AssignOperator::UnsignedShiftRightAssign => Some(">>>="),
        AssignOperator::ElementwiseAndAssign => Some("&="),
        AssignOperator::ElementwiseXorAssign => Some("^="),
        AssignOperator::ElementwiseOrAssign => Some("|="),
        _ => None,
    }
}

impl LintRule for NoBitwise {
    fn meta(&self) -> &'static crate::LintMeta {
        NoBitwise::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let operator = match expression {
                ast::Expression::Binary { operator, .. } => bitwise_binary_text(operator),
                ast::Expression::Unary { operator, .. } => {
                    if matches!(operator, UnaryOperator::ElementwiseNot) {
                        Some("~")
                    } else {
                        None
                    }
                }
                ast::Expression::Assign { operator, .. } => bitwise_assign_text(operator),
                _ => None,
            };

            let Some(operator) = operator else {
                continue;
            };

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
                    format!("bitwise operator `{operator}` is not allowed"),
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
}
