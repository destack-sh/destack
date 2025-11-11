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
        match expression {
            dir::Expression::Path {
                path,
                static_arguments,
            } => {
                let path = self.transpile_path(module, expression_id.into_any(), path, unit);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.transpile_argument(module, *argument, unit))
                        .collect()
                });
                let expression = Expression::Path {
                    path,
                    static_arguments,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::ScalarLiteral { value } => {
                let value = self.transpile_scalar_literal(module, value, unit);
                let expression = Expression::ScalarLiteral { value };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::TupleLiteral { ty: _, elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.tree.get(*element_id))
                    .map(|element| self.transpile_expression(module, element.value(), unit))
                    .collect();
                let expression = Expression::ArrayLiteral { elements };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.tree.get(*element_id))
                    .map(|element| self.transpile_expression(module, element.value(), unit))
                    .collect();
                let expression = Expression::ArrayLiteral { elements };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }

            dir::Expression::TypeUnary { operator, right } => {
                let operator = self.transpile_type_unary_operator(*operator);
                let expression = self.transpile_expression(module, *right, unit);
                let expression = Expression::TypeUnary {
                    operator,
                    right: expression,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::TypeBinary {
                left,
                operator,
                right,
            } => self.transpile_type_binary_expression(
                module,
                expression_id,
                *left,
                *operator,
                *right,
                unit,
            ),
            dir::Expression::Unary { operator, right } => {
                self.transpile_unary_expression(module, expression_id, *operator, *right, unit)
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.transpile_binary_expression(
                module,
                expression_id,
                *left,
                *operator,
                *right,
                unit,
            ),

            _ => panic!("unsupported expression {expression:?}"),
        }
    }
}
