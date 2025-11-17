use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Expression, Generics, Heritage, Mutability, NodeId, Type, TypeLiteral};

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
        generics: &dir::Generics,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Generics> {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|parameter| self.transpile_parameter(module, *parameter, unit))
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
        heritage: &dir::Heritage,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Heritage> {
        let extends_types = heritage
            .extends_types
            .as_ref()
            .map(|extends_types| {
                extends_types
                    .iter()
                    .map(|extends_type| self.transpile_type(module, *extends_type, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let implements_types = heritage
            .implements_types
            .as_ref()
            .map(|implements_types| {
                implements_types
                    .iter()
                    .map(|implements_type| self.transpile_type(module, *implements_type, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let heritage = Heritage {
            extends_types,
            implements_types,
        };
        Ok(heritage)
    }

    /// Transpile a type literal from DIR into JS AST.
    pub fn transpile_type_literal(
        &self,
        ty_id: dir::NodeId<dir::Type>,
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
            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: ty_id.into_any(),
                    message: None,
                });
            }
        };
        Ok(literal)
    }

    // nocheckin

    /// Transpile a type from DIR into JS AST.
    pub fn transpile_type(
        &self,
        module: &'a Module,
        ty_id: dir::NodeId<dir::Type>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Type>> {
        let ty = self.session.tree.get(ty_id);
        let ty = match ty.as_ref() {
            dir::Type::Scalar(scalar) => {
                let literal = self.transpile_type_literal(ty_id, scalar, unit)?;
                Type::Scalar(literal)
            }
            dir::Type::UnresolvedExpression(expression) => {
                let expression = self
                    .transpile_expression(module, *expression, unit)
                    .expect_node::<Expression>(expression.into_any(), unit)?;
                Type::Expression(expression)
            }

            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: ty_id.into_any(),
                    message: None,
                });
            }
        };

        let ty_id = unit.ast.insert_from_source(ty, module.id, ty_id);
        Ok(ty_id)
    }
}
