use destack_dir::{
    AssignOperator, AssignPattern, BinaryOperator, Expression, LocalNodeId, NodeType,
    ProvenanceReason, Tree,
};

use crate::Compiler;

impl Compiler {
    /// Normalize one expression.
    pub(super) fn normalize_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &mut Tree,
    ) {
        let scope = tree.get_scope(expression_id);
        let expression = tree.get(expression_id).clone();
        match expression {
            // compound assignment
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => {
                let binary_operator = assign_operator_to_binary(operator);

                // binary rhs
                let binary_id = tree.reserve_from(
                    NodeType::Expression,
                    expression_id.into_any(),
                    scope,
                    None,
                    Some(ProvenanceReason::Normalized),
                );
                let binary_id: LocalNodeId<Expression> = tree.insert(
                    binary_id,
                    Expression::Binary {
                        left,
                        operator: binary_operator,
                        right,
                    },
                );

                // assignment lhs
                let assign_pattern_id = tree.reserve_from(
                    NodeType::AssignPattern,
                    expression_id.into_any(),
                    scope,
                    Some(expression_id.into_any()),
                    Some(ProvenanceReason::Normalized),
                );
                let assign_pattern_id: LocalNodeId<AssignPattern> =
                    tree.insert(assign_pattern_id, AssignPattern::Expression { value: left });

                // normalized assignment
                tree.replace(
                    expression_id,
                    Expression::Assign {
                        left: assign_pattern_id,
                        right: binary_id,
                    },
                );
            }

            // await maybe
            Expression::AwaitMaybe { expression } => {
                // awaited expression
                let await_id = tree.reserve_from(
                    NodeType::Expression,
                    expression_id.into_any(),
                    scope,
                    None,
                    Some(ProvenanceReason::Normalized),
                );
                let await_id: LocalNodeId<Expression> =
                    tree.insert(await_id, Expression::Await { expression });

                // maybe expression
                tree.replace(expression_id, Expression::Maybe { left: await_id });
            }

            _ => {}
        }
    }
}

/// Convert one assignment operator to its corresponding binary operator.
fn assign_operator_to_binary(op: AssignOperator) -> BinaryOperator {
    match op {
        // multiplication
        AssignOperator::MultiplyAssign => BinaryOperator::Multiply,
        AssignOperator::ExponentAssign => BinaryOperator::Exponent,
        AssignOperator::DivideAssign => BinaryOperator::Divide,
        AssignOperator::RemainderAssign => BinaryOperator::Remainder,

        // addition
        AssignOperator::AddAssign => BinaryOperator::Add,
        AssignOperator::SubtractAssign => BinaryOperator::Subtract,

        // shift
        AssignOperator::ShiftLeftAssign => BinaryOperator::ShiftLeft,
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
    fn test_normalize_assign_binary_to_assign_and_binary() {
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
    fn test_normalize_await_maybe_to_maybe_await() {
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
