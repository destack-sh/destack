use crate::{Compiler, ResolveResult};
use dyst_ast as ast;
use dyst_dir::{
    Expression, NodeId, Type, TypeLiteral, TypeUnaryOperator, UnaryOperator, VarianceBound,
};

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

    /// Lower a VarianceBound to a TypeUnaryOperator.
    pub fn lower_variance_bound(&mut self, bound: ast::VarianceBound) -> VarianceBound {
        match bound {
            ast::VarianceBound::Extends => VarianceBound::Extends,
            ast::VarianceBound::Super => VarianceBound::Super,
        }
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
                Type::Unary {
                    operator: TypeUnaryOperator::Not,
                    right: type_id,
                }
            }
            // maybe
            Expression::Maybe { left } => {
                let type_id = self.try_resolve_expression_to_type(*left)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Maybe,
                    right: type_id,
                }
            }
            // must
            Expression::Must { left } => {
                let type_id = self.try_resolve_expression_to_type(*left)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Must,
                    right: type_id,
                }
            }
            // value
            Expression::Value {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.clone();
                let variance = *variance;
                let type_id = self.try_resolve_expression_to_type(*right)?;
                Type::Value {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // reference
            Expression::Reference {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.clone();
                let variance = *variance;
                let type_id = self.try_resolve_expression_to_type(*right)?;
                Type::Reference {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // binary
            &Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_id = self.try_resolve_expression_to_type(left)?;
                let right_id = self.try_resolve_expression_to_type(right)?;
                Type::Binary {
                    left: left_id,
                    operator,
                    right: right_id,
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
            Expression::TupleLiteral { ty, .. } if ty.is_none() => {
                todo!()
            }
            // struct
            Expression::StructLiteral { ty, .. } if ty.is_none() => {
                todo!()
            }

            // array or slice
            Expression::Index { left, right } => {
                // array with static length
                if let &Some(right) = right {
                    let left_id = self.try_resolve_expression_to_type(*left)?;
                    Type::ArrayStatic {
                        element: left_id,
                        count: right,
                    }
                }
                // slice
                else {
                    let left_id = self.try_resolve_expression_to_type(*left)?;
                    Type::ArraySlice { element: left_id }
                }
            }

            _ => return Ok(None),
        };

        Ok(Some(ty))
    }
}
