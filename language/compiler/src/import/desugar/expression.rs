use destack_dir::{
    AssignOperator, AssignPattern, BinaryOperator, Expression, LocalNodeId, NodeType,
    ProvenanceReason, Tree,
};

use crate::Compiler;

#[allow(clippy::single_match)]
impl Compiler {
    /// Desugar an expression.
    pub(super) fn desugar_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &mut Tree,
    ) {
        let scope = tree.get_scope(expression_id);
        let expression = tree.get(expression_id).clone();
        match expression {
            // AssignBinary -> Assign with Binary expression
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => {
                let binary_operator = assign_operator_to_binary(operator);

                // Binary: left <op> right
                let binary_id = tree.reserve_from(
                    NodeType::Expression,
                    expression_id.into_any(),
                    scope,
                    None,
                    Some(ProvenanceReason::Desugared),
                );
                let binary_id: LocalNodeId<Expression> = tree.insert(
                    binary_id,
                    Expression::Binary {
                        left,
                        operator: binary_operator,
                        right,
                    },
                );

                // wrap the assignment lhs in one assign pattern
                let assign_pattern_id = tree.reserve_from(
                    NodeType::AssignPattern,
                    expression_id.into_any(),
                    scope,
                    Some(expression_id.into_any()),
                    Some(ProvenanceReason::Desugared),
                );
                let assign_pattern_id: LocalNodeId<AssignPattern> =
                    tree.insert(assign_pattern_id, AssignPattern::Expression { value: left });

                // replace AssignBinary with Assign
                tree.replace(
                    expression_id,
                    Expression::Assign {
                        left: assign_pattern_id,
                        right: binary_id,
                    },
                );
            }

            // AwaitMaybe -> Maybe with Await expression
            Expression::AwaitMaybe { expression } => {
                // Await: await expression
                let await_id = tree.reserve_from(
                    NodeType::Expression,
                    expression_id.into_any(),
                    scope,
                    None,
                    Some(ProvenanceReason::Desugared),
                );
                let await_id: LocalNodeId<Expression> =
                    tree.insert(await_id, Expression::Await { expression });

                // replace AwaitMaybe with Maybe
                tree.replace(expression_id, Expression::Maybe { left: await_id });
            }

            _ => {}
        }
    }
}

/// Convert an AssignOperator to its corresponding BinaryOperator.
fn assign_operator_to_binary(op: AssignOperator) -> BinaryOperator {
    match op {
        // multiplication
        AssignOperator::MultiplyAssign => BinaryOperator::Multiply,
        AssignOperator::WrappingMultiplyAssign => BinaryOperator::WrappingMultiply,
        AssignOperator::SaturatingMultiplyAssign => BinaryOperator::SaturatingMultiply,
        AssignOperator::ExponentAssign => BinaryOperator::Exponent,
        AssignOperator::WrappingExponentAssign => BinaryOperator::WrappingExponent,
        AssignOperator::SaturatingExponentAssign => BinaryOperator::SaturatingExponent,
        AssignOperator::DivideAssign => BinaryOperator::Divide,
        AssignOperator::RemainderAssign => BinaryOperator::Remainder,

        // addition
        AssignOperator::AddAssign => BinaryOperator::Add,
        AssignOperator::WrappingAddAssign => BinaryOperator::WrappingAdd,
        AssignOperator::SaturatingAddAssign => BinaryOperator::SaturatingAdd,
        AssignOperator::SubtractAssign => BinaryOperator::Subtract,
        AssignOperator::WrappingSubtractAssign => BinaryOperator::WrappingSubtract,
        AssignOperator::SaturatingSubtractAssign => BinaryOperator::SaturatingSubtract,

        // shift
        AssignOperator::ShiftLeftAssign => BinaryOperator::ShiftLeft,
        AssignOperator::SaturatingShiftLeftAssign => BinaryOperator::SaturatingShiftLeft,
        AssignOperator::ShiftRightAssign => BinaryOperator::ShiftRight,
        AssignOperator::UnsignedShiftRightAssign => BinaryOperator::UnsignedShiftRight,

        // elementwise
        AssignOperator::ElementwiseAndAssign => BinaryOperator::ElementwiseAnd,
        AssignOperator::ElementwiseXorAssign => BinaryOperator::ElementwiseXor,
        AssignOperator::ElementwiseOrAssign => BinaryOperator::ElementwiseOr,

        // boolean
        AssignOperator::AndAssign => BinaryOperator::And,
        AssignOperator::OrAssign => BinaryOperator::Or,
        AssignOperator::CoalesceAssign => BinaryOperator::Coalesce,
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_desugar_assign_binary_to_assign_and_binary() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            "
let x: number = 0;
x += 1;
",
        );
        test.import_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_bound(
            module_id,
            r#"
let x = 0;
x = x + 1;
"#,
        );
    }

    #[test]
    fn test_desugar_await_maybe_to_maybe_await() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
async function test() {
    await? someFallibleAsync();
}
"#,
        );
        test.import_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_bound(
            module_id,
            r#"
async function test() {
    (await someFallibleAsync())?;
}
"#,
        );
    }
}
