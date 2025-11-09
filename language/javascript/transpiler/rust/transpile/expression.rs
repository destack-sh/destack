use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Expression, NodeId};

use crate::{Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a expression from DIR into JS AST.
    pub fn transpile_expression(
        &self,
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> NodeId<Expression> {
        let expression = self.tree.get(expression_id);
        let expression = match expression {
            dir::Expression::ScalarLiteral { value } => {
                let value = self.transpile_scalar_literal(module, value, unit);
                Expression::ScalarLiteral { value }
            }
            _ => panic!("unsupposed expression {expression:?}"),
        };
        unit.ast
            .insert_from_dir(expression, module.id, expression_id)
    }
}
