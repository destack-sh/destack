use dyst_dir::{Expression, NodeId, Type, TypeLiteral, UnaryOperator};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Type (in-place).
    pub fn resolve_type(&mut self, ty_id: NodeId<Type>) -> ResolveResult<()> {
        let ty = self.tree.get(ty_id);
        let Type::UnresolvedExpression(expression_id) = ty else {
            return Ok(());
        };

        // Resolve and update in-place
        let evaluated_ty = self.try_resolve_expression_to_type_value(*expression_id)?;
        let ty = self.tree.get_mut(ty_id);
        *ty = evaluated_ty;

        Ok(())
    }

    /// Try to Resolve an Expression as a Type id.
    fn try_resolve_expression_to_type(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<NodeId<Type>> {
        let ty = self.try_resolve_expression_to_type_value(expression_id)?;
        Ok(self.tree.insert_from_dir(ty, expression_id))
    }

    /// Try to Resolve an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::UnresolvedExpression if it fails.
    fn try_resolve_expression_to_type_value(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<Type> {
        let ty = self
            .resolve_expression_to_type(expression_id)?
            .unwrap_or(Type::UnresolvedExpression(expression_id));
        Ok(ty)
    }

    /// Resolve an Expression into a Type (in-place).
    fn resolve_expression_to_type(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<Option<Type>> {
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
                let type_id = self.try_resolve_expression_to_type(*expression)?;
                Type::Not(type_id)
            }
            // maybe
            Expression::Maybe { left } => {
                let type_id = self.try_resolve_expression_to_type(*left)?;
                Type::Maybe(type_id)
            }
            // must
            Expression::Must { left } => {
                let type_id = self.try_resolve_expression_to_type(*left)?;
                Type::Must(type_id)
            }
            // mutable
            Expression::Type {
                mutability: Some(mutability),
                value,
            } => {
                let mutability = *mutability;
                let type_id = self.try_resolve_expression_to_type(*value)?;
                Type::Mutable {
                    mutability,
                    target: type_id,
                }
            }
            // reference
            Expression::Reference { mutability, right } => {
                let mutability = mutability.clone();
                let type_id = self.try_resolve_expression_to_type(*right)?;
                Type::Reference {
                    mutability,
                    target: type_id,
                }
            }
            // dynamic
            Expression::Dynamic { mutability, right } => {
                let mutability = mutability.clone();
                let type_id = self.try_resolve_expression_to_type(*right)?;
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
                let start_id = self.try_resolve_expression_to_type(start)?;
                let end_id = self.try_resolve_expression_to_type(end)?;
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
                    let left_id = self.try_resolve_expression_to_type(*left)?;
                    Type::Array {
                        element: left_id,
                        count: right,
                    }
                }
                // slice
                else {
                    let left_id = self.try_resolve_expression_to_type(*left)?;
                    Type::Slice { element: left_id }
                }
            }

            _ => return Ok(None),
        };

        Ok(Some(ty))
    }
}
