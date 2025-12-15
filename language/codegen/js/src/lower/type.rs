use crate::{
    CodegenJsError, CodegenJsResult, CodegenJsResultExt, Expression, Generics, Heritage,
    LocalNodeId, ModuleLowerer, Mutability, PrimitiveType, Type, TypeBinaryOperator, TypeLiteral,
    TypeUnaryOperator,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a mutability from DIR into JS AST.
    pub fn lower_mutability(&self, mutability: dir::Mutability) -> Mutability {
        match mutability {
            dir::Mutability::Immutable => Mutability::Immutable,
            dir::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Lower Generics from DIR into JS AST.
    pub fn lower_generics(&mut self, generics: &dir::Generics) -> CodegenJsResult<Generics> {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|parameter| self.lower_parameter(*parameter))
                    .collect::<Result<Vec<_>, CodegenJsError>>()
            })
            .transpose()?;
        let generics = Generics { static_parameters };
        Ok(generics)
    }

    /// Lower Heritage from DIR into JS AST.
    pub fn lower_heritage(&mut self, heritage: &dir::Heritage) -> CodegenJsResult<Heritage> {
        let extends_types = heritage
            .extends_types
            .as_ref()
            .map(|_extends_types| todo!())
            .transpose()?;
        let implements_types = heritage
            .implements_types
            .as_ref()
            .map(|_implements_types| todo!())
            .transpose()?;
        let heritage = Heritage {
            extends_types,
            implements_types,
        };
        Ok(heritage)
    }

    /// Lower a primitive type from DIR into JS AST.
    pub fn lower_primitive_type_value(&self, primitive: dir::PrimitiveType) -> PrimitiveType {
        match primitive {
            dir::PrimitiveType::Boolean => PrimitiveType::Boolean,
            dir::PrimitiveType::Character => PrimitiveType::String,
            dir::PrimitiveType::String => PrimitiveType::String,
            dir::PrimitiveType::Bigint => PrimitiveType::Bigint,
            dir::PrimitiveType::Number => PrimitiveType::Number,
            dir::PrimitiveType::Int(_) => PrimitiveType::Number,
            dir::PrimitiveType::Float(_) => PrimitiveType::Number,
            dir::PrimitiveType::Symbol => PrimitiveType::Symbol,
            dir::PrimitiveType::UniqueSymbol => PrimitiveType::UniqueSymbol,
        }
    }

    /// Lower a primitive type from DIR into JS AST.
    pub fn lower_primitive_type(
        &mut self,
        _ty_id: dir::LocalTypeId,
        primitive: dir::PrimitiveType,
    ) -> CodegenJsResult<PrimitiveType> {
        Ok(self.lower_primitive_type_value(primitive))
    }

    /// Lower a type literal value from DIR into JS AST.
    /// Returns None for unsupported type literals (like Infer, Composite).
    pub fn lower_type_literal_value(&mut self, literal: &dir::TypeLiteral) -> Option<TypeLiteral> {
        let literal = match literal {
            dir::TypeLiteral::Never => TypeLiteral::Never,
            dir::TypeLiteral::Any => TypeLiteral::Any,
            dir::TypeLiteral::Undefined => TypeLiteral::Undefined,
            dir::TypeLiteral::Unknown => TypeLiteral::Unknown,
            dir::TypeLiteral::Void => TypeLiteral::Void,
            dir::TypeLiteral::Null => TypeLiteral::Null,
            dir::TypeLiteral::Primitive(primitive) => {
                let primitive = self.lower_primitive_type_value(*primitive);
                TypeLiteral::Primitive(primitive)
            }
            dir::TypeLiteral::ScalarLiteral(scalar_literal) => {
                let scalar_literal = self.lower_scalar_literal(scalar_literal);
                TypeLiteral::ScalarLiteral(scalar_literal)
            }
            _ => return None,
        };
        Some(literal)
    }

    /// Lower a type literal from DIR into JS AST.
    pub fn lower_type_literal(
        &mut self,
        ty_id: dir::LocalTypeId,
        literal: &dir::TypeLiteral,
    ) -> CodegenJsResult<TypeLiteral> {
        self.lower_type_literal_value(literal).ok_or_else(|| {
            let source_id = self.types.get_type_source(ty_id);
            CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: None,
            }
        })
    }

    /// Lower a type unary operator from DIR into JS AST.
    pub fn lower_type_unary_operator(
        &self,
        ty_id: dir::LocalTypeId,
        operator: dir::TypeUnaryOperator,
    ) -> CodegenJsResult<TypeUnaryOperator> {
        let operator = match operator {
            dir::TypeUnaryOperator::Not => TypeUnaryOperator::Not,
            dir::TypeUnaryOperator::Maybe => TypeUnaryOperator::Maybe,
            dir::TypeUnaryOperator::Must => TypeUnaryOperator::Must,
            dir::TypeUnaryOperator::Type => TypeUnaryOperator::Type,
            dir::TypeUnaryOperator::Newtype => {
                let source_id = self.types.get_type_source(ty_id);
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
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

    /// Lower a type binary operator from DIR into JS AST.
    pub fn lower_type_binary_operator(
        &self,
        _ty_id: dir::LocalTypeId,
        operator: dir::TypeBinaryOperator,
    ) -> CodegenJsResult<TypeBinaryOperator> {
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

    /// Lower a type from DIR into JS AST.
    pub fn lower_type(&mut self, ty_id: dir::LocalTypeId) -> CodegenJsResult<LocalNodeId<Type>> {
        let source_id = self.types.get_type_source(ty_id);
        let ty = self.types.get_type(ty_id);

        let ty_id = match ty {
            dir::Type::TypeLiteral { value: scalar } => {
                let literal = self.lower_type_literal(ty_id, scalar)?;
                let ty = Type::Scalar(literal);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Unevaluated(expression) => {
                let expression = self
                    .lower_expression(*expression)
                    .expect_node::<Expression>(expression.into_global_any(self.module.id), self)?;
                let ty = Type::Expression(expression);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }

            dir::Type::Unary { operator, right } => {
                let operator = self.lower_type_unary_operator(ty_id, *operator)?;
                let right = self.lower_type(*right)?;
                let ty = Type::Unary { operator, right };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_type(*left)?;
                let operator = self.lower_type_binary_operator(ty_id, *operator)?;
                let right = self.lower_type(*right)?;
                let ty = Type::Binary {
                    left,
                    operator,
                    right,
                };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }

            dir::Type::Array { element } => {
                let element = element
                    .map(|element| self.lower_type(element))
                    .transpose()?;
                let ty = Type::Array { element };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Tuple { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = Type::Tuple { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = Type::Union { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Intersection { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = Type::Intersection { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: None,
                });
            }
        };

        Ok(ty_id)
    }
}
