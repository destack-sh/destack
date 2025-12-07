use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, LocalNodeId, LocalTypeId, NodeTree, SymbolTable, Type, TypeLiteral, TypeTable,
    TypeUnaryOperator, UnaryOperator,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Evaluate a Type (in-place).
    /// Converts Type::Unevaluated to the actual Type value.
    pub(super) fn evaluate_type(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let expression_id = {
            let ty = types.get_type(ty_id);
            let Type::Unevaluated(expression_id) = *ty else {
                return Ok(());
            };
            expression_id
        };

        // evaluate and update in-place
        let evaluated_ty = self.try_evaluate_expression_to_type_value(
            module,
            expression_id,
            tree,
            symbols,
            types,
        )?;
        let ty = types.get_type_mut(ty_id);
        *ty = evaluated_ty;

        Ok(())
    }

    /// Try to evaluate an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::Unevaluated if it fails.
    fn try_evaluate_expression_to_type_value(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        let ty = self
            .evaluate_expression_to_type(module, expression_id, tree, symbols, types)?
            .unwrap_or(Type::Unevaluated(expression_id));
        Ok(ty)
    }

    /// Try to evaluate an Expression as a Type id.
    fn try_evaluate_expression_to_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty = self.try_evaluate_expression_to_type_value(
            module,
            expression_id,
            tree,
            symbols,
            types,
        )?;
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Evaluate an Expression into a Type.
    fn evaluate_expression_to_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Type>> {
        let expression = tree.get(expression_id);

        let ty = match expression {
            Expression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            Expression::TypeLiteral { value } => Type::TypeLiteral {
                value: value.clone(),
            },

            // not
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                let type_id =
                    self.try_evaluate_expression_to_type(module, *right, tree, symbols, types)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Not,
                    right: type_id,
                }
            }
            // maybe
            Expression::Maybe { left } => {
                let type_id =
                    self.try_evaluate_expression_to_type(module, *left, tree, symbols, types)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Maybe,
                    right: type_id,
                }
            }
            // must
            Expression::Must { left } => {
                let type_id =
                    self.try_evaluate_expression_to_type(module, *left, tree, symbols, types)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Must,
                    right: type_id,
                }
            }
            // value
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = *mutability;
                let variance = *variance;
                let type_id =
                    self.try_evaluate_expression_to_type(module, *right, tree, symbols, types)?;
                Type::ValueOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // reference
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = *mutability;
                let variance = *variance;
                let type_id =
                    self.try_evaluate_expression_to_type(module, *right, tree, symbols, types)?;
                Type::ReferenceOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // unary
            &Expression::TypeUnary { operator, right } => {
                let right_id =
                    self.try_evaluate_expression_to_type(module, right, tree, symbols, types)?;
                Type::Unary {
                    operator,
                    right: right_id,
                }
            }
            // binary
            &Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_id =
                    self.try_evaluate_expression_to_type(module, left, tree, symbols, types)?;
                let right_id =
                    self.try_evaluate_expression_to_type(module, right, tree, symbols, types)?;
                Type::Binary {
                    left: left_id,
                    operator,
                    right: right_id,
                }
            }

            // tuple (anonymous)
            Expression::TupleExpression { .. } => {
                return Err(AnalyzeError::UnsupportedConstruct {
                    node: expression_id.into_global_any(module.id),
                });
            }
            // object (anonymous)
            Expression::ObjectExpression { .. } => {
                return Err(AnalyzeError::UnsupportedConstruct {
                    node: expression_id.into_global_any(module.id),
                });
            }

            // array or slice
            &Expression::Index { left, right } => {
                // array with static length
                if let Some(right) = right {
                    let left_id =
                        self.try_evaluate_expression_to_type(module, left, tree, symbols, types)?;
                    Type::ArraySized {
                        element: left_id,
                        count: right,
                    }
                }
                // slice
                else {
                    let left_id =
                        self.try_evaluate_expression_to_type(module, left, tree, symbols, types)?;
                    Type::Array {
                        element: Some(left_id),
                    }
                }
            }

            _ => return Ok(None),
        };

        Ok(Some(ty))
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{PrimitiveType, Type, TypeLiteral};

    use crate::TestProgram;

    #[test]
    fn test_analyze_evaluate_type_on_let_expression() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let x: number");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let let_expr_id = module.dir.roots[0];
        let expression = tree.get(let_expr_id);
        let &destack_dir::Expression::Statement {
            statement: let_expr_id,
        } = expression
        else {
            panic!("expected statement");
        };

        let let_ty = types
            .get_declared_type(let_expr_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *let_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
    }
}
