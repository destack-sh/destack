use destack_dir::{AssignOperator, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeType};
use destack_workspace::Module;

use crate::Compiler;

#[allow(clippy::single_match)]
impl Compiler {
    /// Desugare module syntactically: transforms that don't need type information:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    pub(super) fn bind_module_desugar(&self, module: &Module) {
        let mut tree = module.dir.tree.write();

        // desugar expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.desugar_expression(expression_id, &mut tree);
        }
    }

    /// Desugar an expression.
    fn desugar_expression(&self, expression_id: LocalNodeId<Expression>, tree: &mut NodeTree) {
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
                let binary_id =
                    tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
                let binary_id: LocalNodeId<Expression> = tree.insert(
                    binary_id,
                    Expression::Binary {
                        left,
                        operator: binary_operator,
                        right,
                    },
                );

                // replace AssignBinary with Assign
                tree.replace(
                    expression_id,
                    Expression::Assign {
                        left,
                        right: binary_id,
                    },
                );
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
        test.bind_module(module_id);
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
}
