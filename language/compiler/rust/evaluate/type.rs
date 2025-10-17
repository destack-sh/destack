use dyst_dir::{Expression, NodeId, Type, TypeLiteral, UnaryOperator};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Evaluate a Type (in-place).
    pub fn evaluate_type(&mut self, ty_id: NodeId<Type>) -> EvaluateResult<()> {
        let ty = self.tree.get(ty_id);
        let Type::UnevaluatedExpression(expression_id) = ty else {
            return Ok(());
        };

        // evaluate and update in-place
        let evaluated_ty = self.try_evaluate_expression_to_type_value(*expression_id)?;
        let ty = self.tree.get_mut(ty_id);
        *ty = evaluated_ty;

        Ok(())
    }

    /// Try to evaluate an Expression as a Type id.
    fn try_evaluate_expression_to_type(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> EvaluateResult<NodeId<Type>> {
        let ty = self.try_evaluate_expression_to_type_value(expression_id)?;
        Ok(self.tree.insert_from_dir(ty, expression_id))
    }

    /// Try to evaluate an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::UnevaluatedExpression if it fails.
    fn try_evaluate_expression_to_type_value(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> EvaluateResult<Type> {
        let ty = self
            .evaluate_expression_to_type(expression_id)?
            .unwrap_or(Type::UnevaluatedExpression(expression_id));
        Ok(ty)
    }

    /// Evaluate an Expression into a Type (in-place).
    fn evaluate_expression_to_type(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> EvaluateResult<Option<Type>> {
        let expression = self.tree.get(expression_id);

        let ty = match expression {
            Expression::ScalarLiteral { value } => {
                Type::Scalar(TypeLiteral::ScalarLiteral(value.clone()))
            }
            Expression::TypeLiteral { value } => Type::Scalar(value.clone()),

            // not
            Expression::Unary {
                operator: UnaryOperator::Not,
                expression,
            } => {
                let type_id = self.try_evaluate_expression_to_type(*expression)?;
                Type::Not(type_id)
            }
            // maybe
            Expression::Maybe { left } => {
                let type_id = self.try_evaluate_expression_to_type(*left)?;
                Type::Maybe(type_id)
            }
            // must
            Expression::Must { left } => {
                let type_id = self.try_evaluate_expression_to_type(*left)?;
                Type::Must(type_id)
            }
            // reference
            Expression::Reference { mutability, right } => {
                let mutability = mutability.clone();
                let type_id = self.try_evaluate_expression_to_type(*right)?;
                Type::Reference {
                    mutability,
                    target: type_id,
                }
            }
            // dynamic
            Expression::Dynamic { mutability, right } => {
                let mutability = mutability.clone();
                let type_id = self.try_evaluate_expression_to_type(*right)?;
                Type::Dynamic {
                    mutability,
                    target: type_id,
                }
            }

            // range
            &Expression::RangeLiteral {
                start,
                end,
                is_inclusive,
            } => {
                let start_id = self.try_evaluate_expression_to_type(start)?;
                let end_id = self.try_evaluate_expression_to_type(end)?;
                Type::Range {
                    start: start_id,
                    end: end_id,
                    is_inclusive,
                }
            }
            // tuple
            Expression::TupleLiteral { ty, elements } if ty.is_none() => {
                todo!()
            }
            // struct
            Expression::StructLiteral { ty, fields } if ty.is_none() => {
                todo!()
            }

            // array or slice
            Expression::Index { left, right } => {
                // array with static length
                if let &Some(right) = right {
                    let left_id = self.try_evaluate_expression_to_type(*left)?;
                    Type::Array {
                        element: left_id,
                        count: right,
                    }
                }
                // slice
                else {
                    let left_id = self.try_evaluate_expression_to_type(*left)?;
                    Type::Slice { element: left_id }
                }
            }

            _ => return Ok(None),
        };

        Ok(Some(ty))
    }
}
