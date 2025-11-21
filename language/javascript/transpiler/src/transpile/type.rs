use dyst_dir::{self as dir, Module, NodeTree};
use dyst_javascript_ast::{
    Expression, Generics, Heritage, Mutability, LocalNodeId, PrimitiveType, Type, TypeBinaryOperator,
    TypeLiteral, TypeUnaryOperator,
};

use crate::{TranspileError, TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a mutability from DIR into JS AST.
    pub fn transpile_mutability(&self, mutability: dir::Mutability) -> Mutability {
        match mutability {
            dir::Mutability::Immutable => Mutability::Immutable,
            dir::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Transpile Generics from DIR into JS AST.
    pub fn transpile_generics(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        generics: &dir::Generics,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Generics> {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|parameter| self.transpile_parameter(module, tree, *parameter, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let generics = Generics { static_parameters };
        Ok(generics)
    }

    /// Transpile Heritage from DIR into JS AST.
    pub fn transpile_heritage(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        heritage: &dir::Heritage,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Heritage> {
        let extends_types = heritage
            .extends_types
            .as_ref()
            .map(|extends_types| {
                extends_types
                    .iter()
                    .map(|extends_type| self.transpile_type(module, tree, *extends_type, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let implements_types = heritage
            .implements_types
            .as_ref()
            .map(|implements_types| {
                implements_types
                    .iter()
                    .map(|implements_type| {
                        self.transpile_type(module, tree, *implements_type, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let heritage = Heritage {
            extends_types,
            implements_types,
        };
        Ok(heritage)
    }

    /// Transpile a primitive type from DIR into JS AST.
    pub fn transpile_primitive_type(
        &self,
        _module: &'a Module,
        _ty_id: dir::LocalNodeId<dir::Type>,
        primitive: dir::PrimitiveType,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<PrimitiveType> {
        let primitive = match primitive {
            dir::PrimitiveType::Boolean => PrimitiveType::Boolean,
            dir::PrimitiveType::Character => PrimitiveType::String,
            dir::PrimitiveType::String => PrimitiveType::String,
            dir::PrimitiveType::Bigint => PrimitiveType::Bigint,
            dir::PrimitiveType::Number => PrimitiveType::Number,
            dir::PrimitiveType::Int(_) => PrimitiveType::Number,
            dir::PrimitiveType::Float(_) => PrimitiveType::Number,
            dir::PrimitiveType::Symbol => PrimitiveType::Symbol,
            dir::PrimitiveType::UniqueSymbol => PrimitiveType::UniqueSymbol,
        };
        Ok(primitive)
    }

    /// Transpile a type literal from DIR into JS AST.
    pub fn transpile_type_literal(
        &self,
        module: &'a Module,
        ty_id: dir::LocalNodeId<dir::Type>,
        literal: &dir::TypeLiteral,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<TypeLiteral> {
        let literal = match literal {
            dir::TypeLiteral::Never => TypeLiteral::Never,
            dir::TypeLiteral::Any => TypeLiteral::Any,
            dir::TypeLiteral::Undefined => TypeLiteral::Undefined,
            dir::TypeLiteral::Unknown => TypeLiteral::Unknown,
            dir::TypeLiteral::Void => TypeLiteral::Void,
            dir::TypeLiteral::Null => TypeLiteral::Null,
            dir::TypeLiteral::Primitive(primitive) => {
                let primitive = self.transpile_primitive_type(module, ty_id, *primitive, unit)?;
                TypeLiteral::Primitive(primitive)
            }
            dir::TypeLiteral::ScalarLiteral(scalar_literal) => {
                let scalar_literal = self.transpile_scalar_literal(module, scalar_literal, unit);
                TypeLiteral::ScalarLiteral(scalar_literal)
            }
            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: ty_id.into_any(),
                    message: None,
                });
            }
        };
        Ok(literal)
    }

    /// Transpile a type unary operator from DIR into JS AST.
    pub fn transpile_type_unary_operator(
        &self,
        _module: &'a Module,
        ty_id: dir::LocalNodeId<dir::Type>,
        operator: dir::TypeUnaryOperator,
    ) -> TranspileResult<TypeUnaryOperator> {
        let operator = match operator {
            dir::TypeUnaryOperator::Not => TypeUnaryOperator::Not,
            dir::TypeUnaryOperator::Maybe => TypeUnaryOperator::Maybe,
            dir::TypeUnaryOperator::Must => TypeUnaryOperator::Must,
            dir::TypeUnaryOperator::Type => TypeUnaryOperator::Type,
            dir::TypeUnaryOperator::Newtype => {
                return Err(TranspileError::UnsupportedNode {
                    node: ty_id.into_any(),
                    message: None,
                });
            }
            dir::TypeUnaryOperator::Readonly => TypeUnaryOperator::Readonly,
            dir::TypeUnaryOperator::Typeof => TypeUnaryOperator::Typeof,
            dir::TypeUnaryOperator::Keyof => TypeUnaryOperator::Keyof,
            dir::TypeUnaryOperator::Infer => TypeUnaryOperator::Infer,
            dir::TypeUnaryOperator::AsConst => TypeUnaryOperator::AsConst,
            dir::TypeUnaryOperator::Asserts => TypeUnaryOperator::Asserts,
        };
        Ok(operator)
    }

    /// Transpile a type binary operator from DIR into JS AST.
    pub fn transpile_type_binary_operator(
        &self,
        _module: &'a Module,
        _ty_id: dir::LocalNodeId<dir::Type>,
        operator: dir::TypeBinaryOperator,
    ) -> TranspileResult<TypeBinaryOperator> {
        let operator = match operator {
            dir::TypeBinaryOperator::Cast => TypeBinaryOperator::Cast,
            dir::TypeBinaryOperator::In => TypeBinaryOperator::In,
            dir::TypeBinaryOperator::Is => TypeBinaryOperator::Is,
            dir::TypeBinaryOperator::InstanceOf => TypeBinaryOperator::InstanceOf,
            dir::TypeBinaryOperator::Satisfies => TypeBinaryOperator::Satisfies,
            dir::TypeBinaryOperator::Extends => TypeBinaryOperator::Extends,
            dir::TypeBinaryOperator::Implements => TypeBinaryOperator::Implements,
        };
        Ok(operator)
    }

    /// Transpile a type from DIR into JS AST.
    pub fn transpile_type(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        ty_id: dir::LocalNodeId<dir::Type>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Type>> {
        let ty = tree.get(ty_id);

        let ty_id = match ty {
            dir::Type::Scalar(scalar) => {
                let literal = self.transpile_type_literal(module, ty_id, scalar, unit)?;
                let ty = Type::Scalar(literal);
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }
            dir::Type::UnresolvedExpression(expression) => {
                let expression = self
                    .transpile_expression(module, tree, *expression, unit)
                    .expect_node::<Expression>(expression.into_any(), unit)?;
                let ty = Type::Expression(expression);
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }

            dir::Type::Unary { operator, right } => {
                let operator = self.transpile_type_unary_operator(module, ty_id, *operator)?;
                let right = self.transpile_type(module, tree, *right, unit)?;
                let ty = Type::Unary { operator, right };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }
            dir::Type::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.transpile_type(module, tree, *left, unit)?;
                let operator = self.transpile_type_binary_operator(module, ty_id, *operator)?;
                let right = self.transpile_type(module, tree, *right, unit)?;
                let ty = Type::Binary {
                    left,
                    operator,
                    right,
                };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }

            dir::Type::Array { element } => {
                let element = element
                    .map(|element| self.transpile_type(module, tree, element, unit))
                    .transpose()?;
                let ty = Type::Array { element };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }
            dir::Type::Tuple { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.transpile_type(module, tree, *element, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let ty = Type::Tuple { elements };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }
            dir::Type::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.transpile_type(module, tree, *element, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let ty = Type::Union { elements };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }
            dir::Type::Intersection { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.transpile_type(module, tree, *element, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let ty = Type::Intersection { elements };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }
            dir::Type::Function { signature } => {
                let signature = self.transpile_function_signature(module, tree, signature, unit)?;
                let ty = Type::Function { signature };
                unit.ast.insert_from_source(ty, module.id, ty_id)
            }

            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: ty_id.into_any(),
                    message: None,
                });
            }
        };

        Ok(ty_id)
    }
}
