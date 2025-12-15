use destack_dir::{
    AssignOperator, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeType, SymbolTable,
    TypeTable,
};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Desugar a module.
    pub(super) fn desugar_module(&self, module_id: ModuleId) -> ElaborateResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir.tree.write();
        let symbols = module.dir.symbols.read();
        let types = module.dir.types.read();

        // desugar expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.desugar_expression(expression_id, &mut tree, &symbols, &types)?;
        }

        Ok(())
    }

    /// Desugar an expression.
    pub(super) fn desugar_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        let scope = tree.get_scope(expression_id);
        let expression = tree.get(expression_id).clone();
        let replacement = match expression {
            // AssignBinary -> Assign with Binary expression
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => {
                let binary_operator = assign_operator_to_binary(operator);

                // create intermediate Binary: left <op> right
                let binary_id =
                    tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
                let binary = Expression::Binary {
                    left,
                    operator: binary_operator,
                    right,
                };
                let binary_id: LocalNodeId<Expression> = tree.insert(binary_id, binary);

                // create replacement Assign: left = binary
                let assign_id =
                    tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
                let assign = Expression::Assign {
                    left,
                    right: binary_id,
                };
                let assign_id: LocalNodeId<Expression> = tree.insert(assign_id, assign);

                Some(assign_id)
            }

            _ => None,
        };

        // set up alias if we have a replacement (new -> original for reverse lookup)
        if let Some(new_id) = replacement {
            tree.alias_from(new_id.id, expression_id);
        }

        Ok(replacement)
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
    use destack_dir::{BinaryOperator, Expression};

    use crate::tests::TestProgram;

    #[test]
    fn test_desugar_assign_binary_to_assign_and_binary() {
        // AssignBinary (+=) desugars to Assign with nested Binary expression
        let test = TestProgram::memory_sequential();
        test.add_package("test-pkg", None);
        let module_id = test.add_module("test.ds", "
let x: number = 0;
x += 1;
");
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();

        // verify the elaboration produced an Assign with Binary
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();

        // find the elaborated Assign expression
        let mut found_assign_with_binary = false;
        for (_, expr) in tree.iter_nodes_of_type::<Expression>() {
            if let Expression::Assign { right, .. } = expr {
                let right_expr = tree.get(*right);
                if let Expression::Binary { operator, .. } = right_expr
                    && *operator == BinaryOperator::Add
                {
                    found_assign_with_binary = true;
                    break;
                }
            }
        }
        assert!(
            found_assign_with_binary,
            "expected AssignBinary to elaborate into Assign with Binary"
        );
    }
}
