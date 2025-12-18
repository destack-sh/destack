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
        code = "LR005",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoBitwise,
    "Disallow bitwise operators"
}

fn is_bitwise_binary(op: &BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::ShiftLeft
            | BinaryOperator::SaturatingShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::UnsignedShiftRight
            | BinaryOperator::ElementwiseAnd
            | BinaryOperator::ElementwiseXor
            | BinaryOperator::ElementwiseOr
    )
}

fn is_bitwise_assign(op: &AssignOperator) -> bool {
    matches!(
        op,
        AssignOperator::ShiftLeftAssign
            | AssignOperator::SaturatingShiftLeftAssign
            | AssignOperator::ShiftRightAssign
            | AssignOperator::UnsignedShiftRightAssign
            | AssignOperator::ElementwiseAndAssign
            | AssignOperator::ElementwiseXorAssign
            | AssignOperator::ElementwiseOrAssign
    )
}

impl LintRule for NoBitwise {
    fn meta(&self) -> &'static crate::LintMeta {
        NoBitwise::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let is_bitwise = match expression {
                ast::Expression::Binary { operator, .. } => is_bitwise_binary(operator),
                ast::Expression::Unary { operator, .. } => {
                    matches!(operator, UnaryOperator::ElementwiseNot)
                }
                ast::Expression::Assign { operator, .. } => is_bitwise_assign(operator),
                _ => false,
            };

            if !is_bitwise {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_BITWISE.id,
                    NO_BITWISE.code,
                    NO_BITWISE.category,
                    severity,
                    "bitwise operator is not allowed",
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
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a & b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_or() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a | b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_xor() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a ^ b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_not() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = ~a;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_shift_left() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a << b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_shift_right() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a >> b;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_detects_bitwise_assign() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "x &= 1;");
        test.result(result).assert_lint("no-bitwise");
    }

    #[test]
    fn test_allows_logical_and() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a && b;");
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_allows_logical_or() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a || b;");
        test.result(result).assert_no_lint("no-bitwise");
    }

    #[test]
    fn test_allows_arithmetic() {
        let test = TestProgram::for_rule(NoBitwise);
        let result = test.lint_ast("test.ts", "let x = a + b * c;");
        test.result(result).assert_no_lint("no-bitwise");
    }
}
