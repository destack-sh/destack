use destack_dir::{
    AssignOperator, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeType, SymbolTable,
    TypeTable,
};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

#[allow(clippy::single_match)]
impl Compiler {
    /// Desugar a module: syntactic simplification (no type info needed).
    ///
    /// Transforms:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    /// - `RangeExpression` → iterator construction
    /// - `TreeExpression` → runtime construction calls
    /// - `Maybe`/`Must` → explicit error handling
    /// - `Tagged*Expression` → underlying value (newtype erasure)
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

        // desugar annotations
        // TODO: desugar annotations (function annotations into expressions)

        Ok(())
    }

    /// Desugar an expression.
    pub(super) fn desugar_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
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
        };

        Ok(())
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
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
let x = 0;
x = x + 1;
"#,
        );
    }
}
